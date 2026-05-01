^{:nextjournal.clerk/visibility {:code :hide}}
(ns analysis.test
  (:require [nextjournal.clerk :as cl]
            [tablecloth.api :as tc]
            [cheshire.core :as json]
            [analysis.bezier :as bezier]
            [fastmath.stats :as stats]))

; **INFO**: This code may be used to enable the clerk runtime
^{:nextjournal.clerk/visibility #{:hide}}
(cl/code '(do (require '[nextjournal.clerk :as cl])
              (cl/serve! {:browse true :watch-paths ["src"]})))

;; # Analysis of Participant Data

;; # Reading the data

;; Read the dataset as a csv file.
;; After reading, rename all columns to machine workable columns
(def data (-> (tc/dataset "answers.csv")
              (tc/rename-columns {"Zeitstempel" :timestamp
                                  "Kennnummer" :participant-id
                                  "Alter?" :age
                                  "Berufserfahrung in Jahren" :workexperience
                                  "Wie oft Nutzen Sie Virtual Reality Software?" :vrexperience
                                  "Das Präzisionstool \"PRISM, Controller Warping\" hilft, die Kontrollpunkte präzise zu manipulieren" :prism
                                  "Das Erstellen von Hilfskurven im Dreidimensionalen Raum hilft bei der Kontrolle von Punkten" :helper-curves
                                  "Das Manipulieren von Brücken (Spalten und Reihen des Kontrollnetzes) erlaubt eine einfache, präzise großflächige Kontrolle" :bridges
                                  "Durch die Schatten hatte ich das Gefühl, bewegte distanzen sehr gut einschätzen zu können" :shadows
                                  "Das Snapping erlaubt es mir, eine Bewegung rückgängig zu machen" :undo-via-snapping
                                  "Durch die xyz-Box, die beim Bewegen angezeigt wird, habe ich ein verbessertes Verständnis von entfernungen und der Größe von Bewegungen" :xyz-boxes
                                  "Das Ausblenden der Kurve / einblenden von Iso-Linien hilft bei der Manipulation von Kontrollpunkten, die schwer erreichbar sind" :hide-curve
                                  "Das Einfärben der Krümmung bei der räumlichen Visualisierung der Fläche im 3D Raum" :curvature
                                  "Das einblenden von Boxen, und die Variierung der Größen der Boxen verbessert das räumliche Verständnis der Fläche" :boxes
                                  "Durch die NU, NV, NUV Ausrichtung der Boxen erhalte ich ein verbessertes Verständnis der Flächeneingenschaften" :box-alignment
                                  "Orthografische Projektionen erlauben mir das präzise Manipulieren von Kontrollpunkten. Perfektes Anordnen von überlappenden Punkten ist möglich" :orthographic
                                  "Ich würde mir andere Möglichkeiten zur Ausrichtung des Koordinatensystems wünschen (Leer lassen, wenn nicht)" :coordinate-alignmnt
                                  "Das Kombinieren verschiedener Hilfsmittel (PRISM, Brücken, Stepping) hat mir geholfen, das Kontrollnetz nach meinen Wünschen zu manipulieren" :manipulation
                                  "Die Immersion der Virtuellen Realität hilft beim manipulieren von Beziér-Flächen" :immersive-manipulation
                                  "Das Menü war verständlich und einfach zu bedienen" :menu
                                  "Ich habe das Gefühl, Flächen schneller zu modellieren als in Desktop Umbegungen" :faster-than-desktop
                                  "Ich habe das Gefühlt, dass die Flächen Anschaulicher zu betrachten sind, als in Desktop Umgebungen" :immersive-understanding
                                  "Freitext" :free-form-text})))

;; Display data
(cl/table data)

;; # Plotting general Data
;; Start by grouping the ages

(defn category-range
  "Group by categorical values"
  [df column update-fn]
  (-> df
      (tc/select-columns [column :participant-id])
      (tc/group-by column)
      (tc/as-regular-dataset)
      (tc/map-columns :count
                      :data
                      (fn [& rows]
                        (count ((first rows) column))))
      (update-fn)
      (tc/drop-columns [:data :group-id])))

;; Counting categories
(def age-range
  (category-range data :age identity))

(def work-range
  (category-range data :workexperience identity))

(def vr-range
  (category-range data :vrexperience (fn [df] (tc/update-columns df {:name #(map {"Gar Nicht" "Never"
                                                                                  "1 Mal im Monat" "Once a month"}
                                                                                 %)}))))

(cl/table age-range)

(defn display-categorical
  ([data x-label] (display-categorical data x-label x-label))
  ([data x-label legend-label]
   (cl/vl {:data {:values (tc/rows data :as-maps)}
           :width 500
           :heigth 500
           :devicePixelRation 20
           :mark {:type :bar}
           :encoding {:x {:field :name
                          :title x-label}
                      :y {:field :count
                          :type :quantitative
                          :title "Count"
                          :axis {:tickMinStep 1}}
                      :color {:field :name
                              :type :nominal
                              :title legend-label}}})))

(display-categorical age-range "Age in years")

(display-categorical work-range "Work experience in years, specifically for class-A surface software", "Work experience in years")

(display-categorical vr-range "VR experience in years")

(def immersion
  (-> data
      (tc/select-columns [:participant-id :manipulation :immersive-manipulation :immersive-understanding :faster-than-desktop])
      (tc/pivot->longer [:manipulation :immersive-manipulation :immersive-understanding :faster-than-desktop] {:target-columns :immersive-category
                                                                                                               :value-column-name :value})
      (tc/update-columns {:immersive-category #(map {:immersive-manipulation "Immersive Manipulation"
                                                     :manipulation "Overall Manipulation"
                                                     :immersive-understanding "Immersive Understanding"
                                                     :faster-than-desktop "Faster than Desktop"}
                                                    %)})))
(cl/table immersion)

(def tool-usage
  (-> data
      (tc/select-columns [:participant-id :xyz-boxes :hide-curve :curvature :boxes :box-alignment :prism :bridges :shadows :undo-via-snapping])
      (tc/pivot->longer [:xyz-boxes :hide-curve :curvature :boxes :box-alignment :prism :bridges :shadows :undo-via-snapping] {:target-columns :tool-category
                                                                                                                               :value-column-name :value})
      (tc/update-columns {:tool-category #(map {:prism "PRISM Precision Mode"
                                                :bridges "Control row bridges"
                                                :shadows "Start position shadows"
                                                :undo-via-snapping "Undo via Snapping"
                                                :xyz-boxes "XYZ-Boxes to start of drag"
                                                :hide-curve "Hiding the surface"
                                                :curvature "Curvature Color in U/V/UV"
                                                :boxes "Structural Boxes"
                                                :box-alignment "Alignment of structural Boxes"}
                                               %)})))

(cl/table tool-usage)

(def menu
  (-> data
      (tc/select-columns [:participant-id :menu])
      (tc/pivot->longer [:menu] {:target-columns :menu-category
                                 :value-column-name :value})
      (tc/update-columns {:menu-category #(map {:menu "Menu"}
                                               %)})))

(cl/table menu)

(def color-map
  {22144 "#1E77B4"
   34568 "#FF7E0E"
   57849 "#2CA02C"
   88300 "#D72727"
   98562 "#9467BD"
   99461 "#8C564B"})

(defn build-colormap
  [ids-colors-map & {:keys [participant-id] :or {participant-id nil}}]
  (if-not participant-id
    (let [ks (into [] (keys ids-colors-map))]
      {:domain ks
       :range (mapv ids-colors-map ks)})
    {:domain [participant-id]
     :range [(ids-colors-map participant-id)]}))

(defn display-questionaire
  [data fieldname x-title & {:keys [width height scale mean boxplot rules participant-id] :or {width 500
                                                                                               height 1000
                                                                                               scale [-4 4]
                                                                                               mean true
                                                                                               boxplot false
                                                                                               rules true
                                                                                               participant-id nil}}]
  (cl/vl {:data {:values (tc/rows (if-not participant-id
                                    data
                                    (tc/select-rows data (comp #(= % participant-id) :participant-id)))
                                  :as-maps)}
          :width width
          :heigth height
          :devicePixelRation 20
          :transform [{:window [{:op "rank" :as :id_in_group}]
                       :groupby [fieldname :value]}
                      {:calculate "datum.id_in_group == 1 ? 0 : (datum.id_in_group % 2 == 0 ? -1 : 1) * floor(datum.id_in_group / 2)" :as :stacked-rank}]
          :encoding {:x {:field fieldname
                         :type :nominal
                         :title x-title}
                     :y {:field :value
                         :type :quantitative
                         :title "Likert Scale [0,6]. 0 being worst, 6 being best"
                         :scale {:domain [0 6]}
                         :axis {:format :d
                                :tickMinStep 1
                                :values [0 1 2 3 4 5 6]}}}
          :layer [{:mark {:type :point :opacity 0.8 :filled true :size 100 :strokeWidth 2}
                   :encoding {:xOffset {:field :stacked-rank
                                        :scale {:domain scale}
                                        :type :quantitative}
                              :color {:field :participant-id
                                      :type :nominal
                                      :title "Expert ID"
                                      :scale (build-colormap color-map :participant-id participant-id)}
                              :tooltip [{:field :value
                                         :type :quantitative
                                         :title "Likert Value"
                                         :format "d"}]}}
                  (if (and rules (not boxplot))
                    {:mark {:type :rule :opacity 0.08}
                     :encoding {:y {}}}
                    {:mark {:type :point :opacity 0}})
                  (if boxplot
                    {:mark {:type :boxplot :opacity 0.5}}
                    {:mark {:type :point :opacity 0}})
                  (if mean
                    {:mark {:type :point
                            :color :red
                            :shape :diamond
                            :filled false
                            :size 150}
                     :encoding {:y {:field :value
                                    :aggregate :mean
                                    :type :quantitative}
                                :tooltip [{:field :value
                                           :type :quantitative
                                           :aggregate :mean
                                           :title "Mean Score"
                                           :format ".2f"}]}}
                    {:mark {:type :point :opacity 0}})]}))

(display-questionaire immersion :immersive-category "Immersion and Manipulation" :boxplot true)
;; As can be seen, the overall manipulation mean scores are all above 4, except faster than desktop.
;;
;; In the case of "Faster than Desktop", all experts explained this score with missing time to train, and that their normal expert workflow includes working on a desktop.
;; It was noted multiple times, that prolonged time to train might improve the perceived speed difference to classical Desktop environments. (IMPORTANT to note for future work)

(display-questionaire tool-usage :tool-category "Tool Usage" :scale [-3.5 3.5] :boxplot true)
;; Here, it is to note that orthographic cameras where not used. Hence, this question cannot be answered.
;; Especially for the two structural box questions, only 3 out of 6 experts wanted to answer the question. All others decided not to answer the questions, as they didn't really know to score them.
;; The same goes for the "undo via snapping" functionaliy. Almost all experts (except one) did not really like this feature, and deactivated snapping for that reason. Three experts did not score this question.
;;
;; Helper curves where largely ignored. The participants played around with it for one or two minutes, but since the time was short, didn't bother to explore this features in more depth. Hence, not a single participant placed a score on this question.
;;
;; Regarding the hiding of the surface, one of the two experts that scored two or less described, that making the surface transparent instead of hiding would be way better.

(display-questionaire menu :menu-category "Menu Perception" :width 100 :height 1000 :scale [-3 3] :boxplot true)
;; All Experts scored the menu 3 or better. Most really liking the menu. The only downsides found where accidental clicks on the always visible menu, leading to a once occuring situation that needed a restart one minute after starting the evaluation.
;; The other downside was the menus positioning. As the menu is positioned exactly where the VR-Controller is placed in VR-Space, it often occured that the menu was placed way to close to the participant.
;; One participant skillfully used the "degree increase" mode without closing the menu, to speed up the menu usage.

;; ## Display each individual
(map (fn [id]
       (display-questionaire immersion :immersive-category "Immersion and Manipulation" :participant-id id :width 200 :heigth 1000 :mean false)) (tc/column data :participant-id))

(map (fn [id]
       (display-questionaire tool-usage :tool-category "Tool Usage" :participant-id id :scale [-6 6] :width 200 :height 1000 :mean false)) (tc/column data :participant-id))

(map (fn [id]
       (display-questionaire menu :menu-category "Menu Perception" :participant-id id :width 100 :height 1000 :scale [-3 3] :mean false)) (tc/column data :participant-id))

;; # Adding in Evaluation Recorded Data

(def expert-runs
  {11111 "eval_4_7/2026_4_7_11_45.json"
   99461 "eval_4_7/2026_4_7_11_45.json"
   57849 "eval_4_7/2026_4_7_12_14.json"
   22144 "eval_4_7/2026_4_7_13_11.json"
   34568 "eval_4_7/2026_4_7_13_52.json"
   98562 "eval_4_7/2026_4_7_14_37.json"
   88300 "eval_4_7/2026_4_7_15_10.json"})

(defn read-data-sample
  "sample path as input"
  [id expert-runs]
  (let [expert-run (get expert-runs id)]
    (if expert-run
      (get (json/parse-string (slurp expert-run)) "evaluations")
      nil)))

(defn from-m-to-mm
  [x]
  (if x
    (* x 1000)
    x))

(defn from-ms-to-s
  [x]
  (if x
    (/ x 1000)
    x))

(def data-with-eval (tc/add-columns data (apply merge {:filepath (map #(get expert-runs %) (data :participant-id))}
                                                (for [nth (range 3)]
                                                  {(keyword (str "run-" nth "-max")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "max_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-min")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "min_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-avg")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "average_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-time")) (map #(from-ms-to-s (get-in (read-data-sample % expert-runs) [nth "time"])) (data :participant-id))}))))

(cl/table data-with-eval)

(def pivoted
  (tc/pivot->longer data-with-eval (remove #{:participant-id :filepath :free-form-text :coordinate-alignmnt} (vec (tc/column-names data)))
                    {:target-columns :category
                     :value-column-name :value}))

(defn display-participant
  [data participant-id]
  (cl/vl {:data {:values (tc/rows (tc/select-rows data (comp #(= % participant-id) :participant-id)) :as-maps)}
          :width 1000
          :heigth 700
          :devicePixelRation 20
          :encoding {:x {:field :category
                         :type :nominal
                         :title "Manipulation and Immersion"
                         :sort :ascending}
                     :y {:field :value
                         :type :quantitative
                         :title "Likert Scale [0,6]. 0 beeing worst, 6 beeing best"
                         :scale {:domain [0 6]}
                         :axis {:format :d
                                :tickMinStep 1
                                :values [0 1 2 3 4 5 6]}}}

          :layer [{:mark {:type :bar}}]}))

(cl/table pivoted)

(map #(display-participant pivoted %) (tc/column data-with-eval :participant-id))

(defn to-control-point-rows
  "When only corners is true, only return the 4 corners of the resulted nested list"
  [raw-data & {:keys [inverse only-corners] :or {inverse false only-corners false}}]
  (let [rows
        (if (not inverse)
          (->> raw-data
               (sort-by #(vector (get % "x_idx") (get % "y_idx")))
               (group-by #(get % "x_idx"))
               (map #(val %)))

          (->> raw-data
               (sort-by #(vector (get % "y_idx") (get % "y_idx")))
               (group-by #(get % "y_idx"))
               (map #(val %))))]
    (if only-corners
      (vector (first (first rows))
              (last (first rows))
              (first (last rows))
              (last (last rows)))
      rows)))

(defn points-to-xyz-samples-2d
  [pts resolution]
  (let [mapped (mapv (fn [row] (mapv #(get % "point") row))
                     pts)
        params (mapv #(/ % resolution)
                     (take (+ resolution 1) (range)))
        mapped (mapv (fn [u] (mapv (fn [v] (bezier/decas-2d mapped u v)) params)) params)]
    ;; NOTE(jan): Make sure to swap y and z, as the plotting lib uses the z as the vertical axis (bevy used y as vertical)
    {:x (mapv (fn [xs] (mapv #(first %) xs)) mapped)
     :z (mapv (fn [xs] (mapv #(second %) xs)) mapped)
     :y (mapv (fn [xs] (mapv #(nth % 2) xs)) mapped)}))

(defn points-to-xyz-samples-1d
  [pts resolution]
  (for [pts pts]
    (let [mapped (mapv #(get % "point") pts)
          params (mapv #(/ % resolution)
                       (take (+ resolution 1) (range)))
          mapped (mapv #(bezier/decas mapped %) params)]
      {:x (mapv #(first %)  mapped)
       :z (mapv #(second %) mapped)
       :y (mapv #(nth % 2)  mapped)})))

(defn plot-phase
  [data phase]
  (let [recorded-reference-surface (vec (flatten (get-in data [phase "reference_control_points" "Curve" "curves"])))
        recorded-test-surface (get-in data [phase "test_control_points"])
        control-point-rows (to-control-point-rows recorded-reference-surface :inverse true)
        test-point-rows (to-control-point-rows recorded-test-surface)
        samples (points-to-xyz-samples-1d control-point-rows 30)
        samples-test (points-to-xyz-samples-2d test-point-rows 30)]
    (cl/plotly {:data (conj (mapv #(merge {:type :scatter3d
                                           :mode :lines
                                           :opacity 1
                                           :line {:width 6}}
                                          %) samples)
                            {:type :surface
                             :x (:x samples-test)
                             :y (:y samples-test)
                             :z (:z samples-test)
                             :colorbar {:x 0
                                        :len 0.6
                                        :y 0.5
                                        :thickness 20}})
                :config {:displayModeBar false}})))

(defn print-all-phases
  [participant]
  (let [recorded-data-sample (read-data-sample participant expert-runs)]
    (for [phase (range (count recorded-data-sample))]
      (plot-phase recorded-data-sample phase))))

(defn eval-min-max-avg-single-control-point-distance
  [filedata phase]
  (if (get filedata phase)
    (let [reference-surface-data (vec (flatten (get-in filedata [phase "reference_control_points" "Curve" "curves"])))
          test-surface-data (get-in filedata [phase "test_control_points"])
          reference-control-points (flatten (to-control-point-rows reference-surface-data :inverse true))
          test-control-points (flatten (to-control-point-rows test-surface-data :only-corners true))
          reference-raw-points (mapv #(get % "point") reference-control-points)
          test-raw-points (mapv #(get % "point") test-control-points)
          point-assignments (bezier/greedy-min-assignment test-raw-points reference-raw-points)]
      (if (seq point-assignments)
        {:min (* 1000 (reduce (fn [c v] (min c (:dist v))) Double/MAX_VALUE (vals point-assignments)))
         :max (* 1000 (reduce (fn [c v] (max c (:dist v))) Double/MIN_VALUE (vals point-assignments)))
         :avg (* 1000 (/ (reduce (fn [c v] (+ c (:dist v))) 0 (vals point-assignments)) (count point-assignments)))}
        nil))
    nil))

(defn add-min-max-avg-control-point-distance
  [df phase]
  (let [participants (get df :participant-id)
        filedata (map #(read-data-sample % expert-runs) participants)
        min-max-avg (map #(eval-min-max-avg-single-control-point-distance % phase) filedata)]
    (tc/add-columns df {(keyword (str "run-" phase "-cp-min-dist")) (map :min min-max-avg)
                        (keyword (str "run-" phase "-cp-max-dist")) (map :max min-max-avg)
                        (keyword (str "run-" phase "-cp-avg-dist")) (map :avg min-max-avg)})))

(def df-with-cp-dist (-> data-with-eval
                         (add-min-max-avg-control-point-distance 0)
                         (add-min-max-avg-control-point-distance 1)))

(cl/table df-with-cp-dist)

(def df-with-cp-dist-filtered
  (tc/select-rows df-with-cp-dist (comp #(not (= % 34568)) :participant-id)))

(def df-with-cp-dist-filtered-run-2
  (tc/select-rows df-with-cp-dist (comp #(not (#{34568 98562 88300} %)) :participant-id)))

(def df-with-cp-dist-filtered-run-2-1
  (tc/select-rows df-with-cp-dist (comp #(not (#{34568 98562 88300 57849} %)) :participant-id)))

(cl/table df-with-cp-dist-filtered)

(def run-0-pivoted
  (-> df-with-cp-dist
      (tc/select-columns [:participant-id
                          :run-0-min :run-0-max :run-0-avg
                          :run-0-cp-min-dist :run-0-cp-max-dist :run-0-cp-avg-dist])
      (tc/pivot->longer [:run-0-min :run-0-max :run-0-avg
                         :run-0-cp-min-dist :run-0-cp-max-dist :run-0-cp-avg-dist] {:target-columns :run-category
                                                                                    :value-column-name :value})
      (tc/update-columns {:run-category #(map {:run-0-min "Min"
                                               :run-0-max "Max"
                                               :run-0-avg "Avg"
                                               :run-0-cp-min-dist "Min Distance to Control Point"
                                               :run-0-cp-max-dist "Max Distance to Control Point"
                                               :run-0-cp-avg-dist "Avg Distance to Control Point"}
                                              %)})))

(def run-1-pivoted
  (-> df-with-cp-dist-filtered
      (tc/select-columns [:participant-id
                          :run-1-min :run-1-max :run-1-avg
                          :run-1-cp-min-dist :run-1-cp-max-dist :run-1-cp-avg-dist])
      (tc/pivot->longer [:run-1-min :run-1-max :run-1-avg
                         :run-1-cp-min-dist :run-1-cp-max-dist :run-1-cp-avg-dist] {:target-columns :run-category
                                                                                    :value-column-name :value})
      (tc/update-columns {:run-category #(map {:run-1-min "Min"
                                               :run-1-max "Max"
                                               :run-1-avg "Avg"
                                               :run-1-cp-min-dist "Min Distance to Control Point"
                                               :run-1-cp-max-dist "Max Distance to Control Point"
                                               :run-1-cp-avg-dist "Avg Distance to Control Point"}
                                              %)})))

(def run-2-pivoted
  (-> df-with-cp-dist-filtered-run-2
      (tc/select-columns [:participant-id
                          :run-2-min :run-2-max :run-2-avg])
      (tc/pivot->longer [:run-2-min :run-2-max :run-2-avg] {:target-columns :run-category
                                                            :value-column-name :value})
      (tc/update-columns {:run-category #(map {:run-2-min "Min"
                                               :run-2-max "Max"
                                               :run-2-avg "Avg"}
                                              %)})))

(def run-2-1-pivoted
  (-> df-with-cp-dist-filtered-run-2-1
      (tc/select-columns [:participant-id
                          :run-2-min :run-2-max :run-2-avg])
      (tc/pivot->longer [:run-2-min :run-2-max :run-2-avg] {:target-columns :run-category
                                                            :value-column-name :value})
      (tc/update-columns {:run-category #(map {:run-2-min "Min"
                                               :run-2-max "Max"
                                               :run-2-avg "Avg"}
                                              %)})))

(def runs-time-pivoted
  (-> df-with-cp-dist-filtered
      (tc/select-columns [:participant-id
                          :run-0-time :run-1-time :run-2-time])
      (tc/pivot->longer [:run-0-time :run-1-time :run-2-time] {:target-columns :run-category
                                                               :value-column-name :value})
      (tc/update-columns {:run-category #(map {:run-0-time "Run 1 Time"
                                               :run-1-time "Run 2 Time"
                                               :run-2-time "Run 3 Time"}
                                              %)})))

(cl/table run-0-pivoted)
(defn plot-run-pivoted
  [run run-count & {:keys [width height scale mean boxplot rules participant-id] :or {width 300
                                                                                      height 500
                                                                                      scale [-4 4]
                                                                                      mean true
                                                                                      boxplot false
                                                                                      rules true
                                                                                      participant-id nil}}]
  (cl/vl {:data {:values (tc/rows run :as-maps)}
          :width width
          :heigth height
          :devicePixelRation 20
          :transform [{:window [{:op "rank" :as :id_in_group}]
                       :groupby [:run-category :value]}
                      {:calculate "datum.id_in_group == 1 ? 0 : (datum.id_in_group % 2 == 0 ? -1 : 1) * floor(datum.id_in_group / 2)" :as :stacked-rank}]
          :encoding {:x {:field :run-category
                         :type :nominal
                         :title (str "Run " run-count " Distances")
                         :sort ["Min" "Avg" "Max" "Min Distance to Control Point" "Avg Distance to Control Point" "Max Distance to Control Point"]}
                     :y {:field :value
                         :type :quantitative
                         :title "Distance in mm to the reference curves"}}

          :layer [{:mark {:type :point :opacity 0.8 :filled true :size 100 :strokeWidth 2}
                   :encoding {:xOffset {:field :stacked-rank
                                        :scale {:domain scale}
                                        :type :quantitative}
                              :color {:field :participant-id
                                      :type :nominal
                                      :title "Expert ID"
                                      :scale (build-colormap color-map :participant-id participant-id)}
                              :tooltip [{:field :value
                                         :type :quantitative
                                         :title "Distance im mm"
                                         :format ".3f"}]}}
                  (if (and rules (not boxplot))
                    {:mark {:type :rule :opacity 0.08}
                     :encoding {:y {}}}
                    {:mark {:type :point :opacity 0}})
                  (if boxplot
                    {:mark {:type :boxplot :opacity 0.5}}
                    {:mark {:type :point :opacity 0}})
                  (if mean
                    {:mark {:type :point
                            :color :red
                            :shape :diamond
                            :filled false
                            :size 150}
                     :encoding {:y {:field :value
                                    :aggregate :mean
                                    :type :quantitative}
                                :tooltip [{:field :value
                                           :type :quantitative
                                           :aggregate :mean
                                           :title "Mean Score"
                                           :format ".2f"}]}}
                    {:mark {:type :point :opacity 0}})]}))

(plot-run-pivoted run-0-pivoted 1 :boxplot true)
(plot-run-pivoted run-1-pivoted 2 :boxplot true)
(plot-run-pivoted run-2-pivoted 3)
(plot-run-pivoted run-2-1-pivoted 3 :mean false)

(defn plot-time-pivoted
  [times-pivoted & {:keys [width height scale mean boxplot rules participant-id] :or {width 200
                                                                                      height 500
                                                                                      scale [-4 4]
                                                                                      mean true
                                                                                      boxplot false
                                                                                      rules true
                                                                                      participant-id nil}}]
  (cl/vl {:data {:values (tc/rows times-pivoted :as-maps)}
          :width width
          :heigth height
          :devicePixelRation 20
          :transform [{:window [{:op "rank" :as :id_in_group}]
                       :groupby [:run-category :value]}
                      {:calculate "datum.id_in_group == 1 ? 0 : (datum.id_in_group % 2 == 0 ? -1 : 1) * floor(datum.id_in_group / 2)" :as :stacked-rank}]
          :encoding {:x {:field :run-category
                         :type :nominal
                         :title "Time to completion"
                         :sort ["Run 0 Time" "Run 1 Time" "Run 2 Time"]}
                     :y {:field :value
                         :type :quantitative
                         :title "Time in Seconds"}}

          :layer [{:mark {:type :point :opacity 0.8 :filled true :size 100 :strokeWidth 2}
                   :encoding {:xOffset {:field :stacked-rank
                                        :scale {:domain scale}
                                        :type :quantitative}
                              :color {:field :participant-id
                                      :type :nominal
                                      :title "Expert ID"
                                      :scale (build-colormap color-map :participant-id participant-id)}
                              :tooltip [{:field :value
                                         :type :quantitative
                                         :title "Time in seconds"
                                         :format ".2f"}]}}
                  (if (and rules (not boxplot))
                    {:mark {:type :rule :opacity 0.08}
                     :encoding {:y {}}}
                    {:mark {:type :point :opacity 0}})
                  (if boxplot
                    {:mark {:type :boxplot :opacity 0.5}}
                    {:mark {:type :point :opacity 0}})
                  (if mean
                    {:mark {:type :point
                            :color :red
                            :shape :diamond
                            :filled false
                            :size 150}
                     :encoding {:y {:field :value
                                    :aggregate :mean
                                    :type :quantitative}
                                :tooltip [{:field :value
                                           :type :quantitative
                                           :aggregate :mean
                                           :title "Mean Score"
                                           :format ".2f"}]}}
                    {:mark {:type :point :opacity 0}})]}))
(plot-time-pivoted runs-time-pivoted :boxplot true)

;; This code may be used to calculate an correlation matrix, however,
;; it does not seem like there is enough data to even try.
;; Its worth noting, that a sample size of 6 is very small.
;; It does not help, that some questionaire values are nil,
;; and run 3 (run-2-*) was not always complete.
;; Hence the correlation is an empty list in this case
(def corr-matrix
  (stats/correlation-matrix (list
                            ;; (tc/column df-with-cp-dist :prism)
                            ;; (tc/column df-with-cp-dist :helper-curves)
                            ;; (tc/column df-with-cp-dist :bridges)
                            ;; (tc/column df-with-cp-dist :shadows)
                            ;; (tc/column df-with-cp-dist :undo-via-snapping)
                            ;; (tc/column df-with-cp-dist :xyz-boxes)
                            ;; (tc/column df-with-cp-dist :hide-curve)
                            ;; (tc/column df-with-cp-dist :curvature)
                            ;; (tc/column df-with-cp-dist :boxes)
                            ;; (tc/column df-with-cp-dist :box-alignment)
                            ;; (tc/column df-with-cp-dist :orthographic)
                            ;; (tc/column df-with-cp-dist :manipulation)
                            ;; (tc/column df-with-cp-dist :menu)
                             (tc/column df-with-cp-dist :immersive-understanding)
                             (tc/column df-with-cp-dist :immersive-manipulation)
                             (tc/column df-with-cp-dist :manipulation)

                             (tc/column df-with-cp-dist :run-0-min)
                             (tc/column df-with-cp-dist :run-0-avg)
                             (tc/column df-with-cp-dist :run-0-max)
                             (tc/column df-with-cp-dist :run-0-time)
                             (tc/column df-with-cp-dist :run-0-cp-min-dist)
                             (tc/column df-with-cp-dist :run-0-cp-avg-dist)
                             (tc/column df-with-cp-dist :run-0-cp-max-dist))))

                           ;; (tc/column df-with-cp-dist :run-1-min)
                           ;; (tc/column df-with-cp-dist :run-1-avg)
                           ;; (tc/column df-with-cp-dist :run-1-max)
                           ;; (tc/column df-with-cp-dist :run-1-time)
                           ;; (tc/column df-with-cp-dist :run-1-cp-min-dist)
                           ;; (tc/column df-with-cp-dist :run-1-cp-avg-dist)
                           ;; (tc/column df-with-cp-dist :run-1-cp-avg-dist)))

                           ;; (tc/column df-with-cp-dist :run-2-min)
                           ;; (tc/column df-with-cp-dist :run-2-avg)
                           ;; (tc/column df-with-cp-dist :run-2-max)
                           ;; (tc/column df-with-cp-dist :run-2-time)))

(def labels ["Immersive Understanding" "Immersive Manipulation" "Overall Manipulation" "Min" "Avg" "Max" "Time" "CP Min" "CP Avg" "CP Max"])
(def tidy-data
  (flatten
   (map-indexed
    (fn [i row]
      (map-indexed
       (fn [j val]
         {:x (nth labels i) :y (nth labels j) :corr val})
       row))
    corr-matrix)))

(def heatmap-spec
  {:data {:values tidy-data}
   :mark "rect"
   :devicePixelRation 20
   :encoding {:x {:field "x" :type "nominal" :title nil :sort :ascending}
              :y {:field "y" :type "nominal" :title nil :sort :ascending}
              :color {:field "corr"
                      :type "quantitative"
                      :scale {:domain [-1 1] :scheme "redblue" :reverse true}
                      :title "Pearson Corr"}
              :tooltip [{:field "x"} {:field "y"} {:field "corr"}]}
   :config {:axis {:grid false}
            :view {:stroke nil}}})

(cl/vl heatmap-spec)

(def corr-matrix
  (stats/correlation-matrix (list
                            ;; (tc/column df-with-cp-dist :prism)
                            ;; (tc/column df-with-cp-dist :helper-curves)
                            ;; (tc/column df-with-cp-dist :bridges)
                            ;; (tc/column df-with-cp-dist :shadows)
                            ;; (tc/column df-with-cp-dist :undo-via-snapping)
                            ;; (tc/column df-with-cp-dist :xyz-boxes)
                            ;; (tc/column df-with-cp-dist :hide-curve)
                            ;; (tc/column df-with-cp-dist :curvature)
                            ;; (tc/column df-with-cp-dist :boxes)
                            ;; (tc/column df-with-cp-dist :box-alignment)
                            ;; (tc/column df-with-cp-dist :orthographic)
                            ;; (tc/column df-with-cp-dist :manipulation)
                            ;; (tc/column df-with-cp-dist :menu)
                             (tc/column df-with-cp-dist :immersive-understanding)
                             (tc/column df-with-cp-dist :immersive-manipulation)
                             (tc/column df-with-cp-dist :manipulation)

                             ;; (tc/column df-with-cp-dist :run-0-min)
                             ;; (tc/column df-with-cp-dist :run-0-avg)
                             ;; (tc/column df-with-cp-dist :run-0-max)
                             ;; (tc/column df-with-cp-dist :run-0-time)
                             ;; (tc/column df-with-cp-dist :run-0-cp-min-dist)
                             ;; (tc/column df-with-cp-dist :run-0-cp-avg-dist)
                             ;; (tc/column df-with-cp-dist :run-0-cp-max-dist))

                             (tc/column df-with-cp-dist :run-1-min)
                             (tc/column df-with-cp-dist :run-1-avg)
                             (tc/column df-with-cp-dist :run-1-max)
                             (tc/column df-with-cp-dist :run-1-time)
                             (tc/column df-with-cp-dist :run-1-cp-min-dist)
                             (tc/column df-with-cp-dist :run-1-cp-avg-dist)
                             (tc/column df-with-cp-dist :run-1-cp-max-dist))))

                           ;; (tc/column df-with-cp-dist :run-2-min)
                           ;; (tc/column df-with-cp-dist :run-2-avg)
                           ;; (tc/column df-with-cp-dist :run-2-max)
                           ;; (tc/column df-with-cp-dist :run-2-time)))

(def tidy-data
  (flatten
   (map-indexed
    (fn [i row]
      (map-indexed
       (fn [j val]
         {:x (nth labels i) :y (nth labels j) :corr val})
       row))
    corr-matrix)))

(def heatmap-spec
  {:data {:values tidy-data}
   :mark "rect"
   :encoding {:x {:field "x" :type "nominal" :title nil :sort :ascending}
              :y {:field "y" :type "nominal" :title nil :sort :ascending}
              :color {:field "corr"
                      :type "quantitative"
                      :scale {:domain [-1 1] :scheme "redblue" :reverse true}
                      :title "Pearson Corr"}
              :tooltip [{:field "x"} {:field "y"} {:field "corr"}]}
   :config {:axis {:grid false}
            :view {:stroke nil}}})

(cl/vl heatmap-spec)
;; # Analytical statements
;; General statements on analytical results. Statistical Analysis is not possible
;;
;; ## Orthographic projection
;; The orthographic projection was never used. No expert expressed the need for orthographic projections during tests.
;; Hence, no participant could answer the questionaire in that regard.

;; # Statements
;; ## 99461
;;
;; - 16-18: Kugeln ziehen ist ein Problem
;; - 19: Sensoren großes Problem. Alignment Wichtig
;; - Aufpassen, dass Snapping aus ist
;; - Proband bewegt sich dynamisch im Raum
(print-all-phases 99461)
;;
;; ## 57849
;;
;; - Hat nicht viel geredet, nur freitext kommentar am Ende dargelassen
(print-all-phases 57849)
;;
;; ## 22144
;;
;; - Man braucht nur einen Controller
;; - Haken auch mit Kugel zum besseren anpeilen
;; - Beim loslassen des Controllers verspringt das arg
;; - Experte, hat super schnell alles verstanden und gemacht
(print-all-phases 22144)
;;
;; ## 34568
;;
;; - Durchaus inituitiv
;; - Menü Platzierung gefährlich. Man klickt oft drauf, wenn man nicht aufpasst
;; - Kugel verdeckt den Eckpunkt, daher MUSS ein Scaler rein, der aber nicht die Bewegung einschränkt
;; - Punkt Transparent, damit Mittelpunkt hochgenau angezeigt wird
;; - Keine genaueren Kontrollmöglichkeiten, Ränder stimmen nicht, ist nicht ersichtlich, da zu große Kugeln
;; - Fehlen von minimalen kleinen Bewegungen
;; - Kommt sehr oft auf das dauerhafte Menü. Vielleicht das Menü auch ausblenden
;; - Kriege das gar nicht genau hingeschoben. Deckungsgleichheit lediglich über Augenmaß
;; - Kugeln anfassen ist ok. Intuitiv
;; - Loslassen der Kugeln trotz Präzisionsmodus verreißt
;; - Boxen besser Durchsichtig bei Ziehen
;; - Daumenmenü immer zu nah am Körper, besser fixe distanze
(print-all-phases 34568)
;;
;; ## 98562
;;
;; - Man geht ungern in die Fläche Rein, (intuitiv)
;; - Es ist gut mit dem dicht heran gehen, dann kann man genauer arbeiten. Dann ist das Wackeln nicht mehr so schlimm
;; - Sensoren sind das A. und O. schlechte sensorik, macht die Precision zu nichte
;; - Iso Linien müssen auf jeden Fall dicker
;; - Kügelchen helfen auf jedenfall bei der Orientierung
;; - Dicht genug rangehen, lässt sich aber gut bewegen. Präzise == dicht heran
;; - Entfernung wirkt natürlich, auch die Bewegung
;; - Vielleicht unsichtbar machen auch nicht unbedingt hilfreich, aber kleiner
;; - Bei Kugel ein kleines Koordinatensystem einblenden (beim Ziehen)
;; - Man müsste testen, welche features etwas ausmachen, oder egal sind, indem man einen gegentest machet
;; - Iso linien zu klein
;; - Fläche vielleicht ein bisschen durchsichtig
;; - AR hilft, nirgends gegen zu laufen
;; - AR könnte geholfen haben, gegen VR Sickness zu arbeiten
;; - Vielleicht Raum der größe nach anpassen. Aufgeräumt
;; - Menü war zu nar dran, sonst gut
;; - GGF wenn man viel Übung hat. War aber nicht schneller, als am Desktop. Man konnte direkt an die Kontrollpunkte, und konnte die viel besser Visualisieren. Ggf sogar präziser als am Desktop
(print-all-phases 98562)
;;
;; ## 88300
;;
;; - Irritierend durch die Fläche zu laufen
;; - Wäre cool, wenn die Control Sphere transparent wäre
;; - Robots immer zu dem "Spieler" gerichtet
;; - Reihen Anfasser lieber als Zylinder
;; - Wenn boxen, dann nur NUV
;; - Menü zu nah am Körper
;; - Menü zu unübersichtlich
(print-all-phases 88300)

;; ## Most important Statements
;; - It is irritating to walk into the surface/path (2x)
;; - It is nice to move very close to control points, both for precision and perception (1x)
;; - It would help to hide the control point (or make it tiny) to precisely see the center of the control point, for precision (3x)
;; - The menu is too close to the body (2x)
;; - Might actually be more precise than on desktop (1x)
;;

;; ## Other Mentions
;; - Almost all experts moved dynamically around, some also kneeling or bowing down
;; - Every Participant could instantly work with control point manipulation. The act of grabbing, clicking and dragging seemd very intuitive
;; - Often it occured that the VR-Setup lost track of the controls, this lead to short frustrations and jittering of the controler
;; - Even though PRISM helped for some participants, other claimed it is useless. Quite often, releasing the trigger of the controller lead to control point movement, even though, the prism mode was on.
;; This indicates wrong thresholds (6x, happened every time)

;; # Evaluation Flaws
;; The evaluation is flawed, as not all tools where neccessary for completion of the tasks.
;; Additionally, the short available timeslot may lead to missing opportunities to explore the software freely.
;; Especially the third task was often cut short.
