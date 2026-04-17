^{:nextjournal.clerk/visibility {:code :hide}}
(ns analysis.test
  (:require [nextjournal.clerk :as cl]
            [tablecloth.api :as tc]
            [cheshire.core :as json]
            [analysis.bezier :as bezier]))

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
      (tc/select-columns [:participant-id :prism :bridges :shadows :undo-via-snapping :xyz-boxes :hide-curve :curvature :boxes :box-alignment])
      (tc/pivot->longer [:prism :bridges :shadows :undo-via-snapping :xyz-boxes :hide-curve :curvature :boxes :box-alignment] {:target-columns :tool-category
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

(cl/vl {:data {:values (tc/rows immersion :as-maps)}
        :width 500
        :heigth 500
        :encoding {:x {:field :immersive-category
                       :type :nominal
                       :title "Manipulation and Immersion"}
                   :y {:field :value
                       :type :quantitative
                       :title "Likert Scale [0,6]. 0 beeing worst, 6 beeing best"}}

        :layer [{:mark {:type :boxplot}}
                {:mark {:type :point
                        :color :red
                        :shape :diamond}
                 :encoding {:y {:field :value
                                :aggregate :mean
                                :type :quantitative}
                            :tooltip [{:field :value
                                       :type :quantitative
                                       :aggregate :mean
                                       :title "Mean Score"
                                       :format ".2f"}]}}]})

;; As can be seen, the overall manipulation mean scores are all above 4, except faster than desktop.
;;
;; In the case of "Faster than Desktop", all experts explained this score with missing time to train, and that their normal expert workflow includes working on a desktop.
;; It was noted multiple times, that prolonged time to train might improve the perceived speed difference to classical Desktop environments. (IMPORTANT to note for future work)

(cl/vl {:data {:values (tc/rows tool-usage :as-maps)}
        :width 500
        :heigth 500
        :encoding {:x {:field :tool-category
                       :type :nominal
                       :title "Tool Usage"}
                   :y {:field :value
                       :type :quantitative
                       :title "Likert Scale [0,6]. 0 beeing worst, 6 beeing best"}}

        :layer [{:mark {:type :boxplot}}
                {:mark {:type :point
                        :color :red
                        :shape :diamond}
                 :encoding {:y {:field :value
                                :aggregate :mean
                                :type :quantitative}
                            :tooltip [{:field :value
                                       :type :quantitative
                                       :aggregate :mean
                                       :title "Mean Score"
                                       :format ".2f"}]}}]})

(cl/table data)

;; Here, it is to note that orthographic cameras where not used. Hence, this question cannot be answered.
;; Especially for the two structural box questions, only 3 out of 6 experts wanted to answer the question. All others decided not to answer the questions, as they didn't really know to score them.
;; The same goes for the "undo via snapping" functionaliy. Almost all experts (except one) did not really like this feature, and deactivated snapping for that reason. Three experts did not score this question.
;;
;; Helper curves where largely ignored. The participants played around with it for one or two minutes, but since the time was short, didn't bother to explore this features in more depth. Hence, not a single participant placed a score on this question.
;;
;; Regarding the hiding of the surface, one of the two experts that scored two or less described, that making the surface transparent instead of hiding would be way better.
;;

(cl/vl {:data {:values (tc/rows menu :as-maps)}
        :width 100
        :heigth 500
        :encoding {:x {:field :menu-category
                       :type :nominal
                       :title "Menu usefullness"}
                   :y {:field :value
                       :type :quantitative
                       :title "Likert Scale [0,6]. 0 beeing worst, 6 beeing best"}}

        :layer [{:mark {:type :boxplot}}
                {:mark {:type :point
                        :color :red
                        :shape :diamond}
                 :encoding {:y {:field :value
                                :aggregate :mean
                                :type :quantitative}
                            :tooltip [{:field :value
                                       :type :quantitative
                                       :aggregate :mean
                                       :title "Mean Score"
                                       :format ".2f"}]}}]})

;; All Experts scored the menu 3 or better. Most really liking the menu. The only downsides found where accidental clicks on the always visible menu, leading to a once occuring situation that needed a restart one minute after starting the evaluation.
;; The other downside was the menus positioning. As the menu is positioned exactly where the VR-Controller is placed in VR-Space, it often occured that the menu was placed way to close to the participant.
;; One participant skillfully used the "degree increase" mode without closing the menu, to speed up the menu usage.

;; # Adding in Evaluation Recorded Data

(def expert-runs
  {11111 "evals/2026_4_1_12_19.json"
   99461 "evals/2026_4_1_12_19.json"
   57849 "evals/2026_4_1_12_19.json"
   34568 "evals/2026_4_1_12_19.json"
   98562 "evals/2026_4_1_12_19.json"
   88300 "evals/2026_4_1_12_19.json"
   22144 "evals/2026_4_1_12_19.json"})

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

(def data-with-eval (tc/add-columns data (apply merge {:filepath (map #(get expert-runs %) (data :participant-id))}
                                                (for [nth (range 3)]
                                                  {(keyword (str "run-" nth "-max")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "max_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-min")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "min_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-avg")) (map #(from-m-to-mm (get-in (read-data-sample % expert-runs) [nth "average_dist"])) (data :participant-id))
                                                   (keyword (str "run-" nth "-time")) (map #(get-in (read-data-sample % expert-runs) [nth "time"]) (data :participant-id))}))))

(cl/table data-with-eval)

(def recorded-data-sample
  (read-data-sample 11111 expert-runs))

(defn to-control-point-rows
  [raw-data & {:keys [inverse] :or {inverse false}}]
  (if (not inverse)
    (->> raw-data
         (sort-by #(vector (get % "x_idx") (get % "y_idx")))
         (group-by #(get % "x_idx"))
         (map #(val %)))

    (->> raw-data
         (sort-by #(vector (get % "y_idx") (get % "y_idx")))
         (group-by #(get % "y_idx"))
         (map #(val %)))))

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

(for [phase (range (count recorded-data-sample))]
  (plot-phase recorded-data-sample phase))

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
;;
;; ## 57849
;;
;; - Hat nicht viel geredet, nur freitext kommentar am Ende dargelassen
;;
;; ## 22144
;;
;; - Man braucht nur einen Controller
;; - Haken auch mit Kugel zum besseren anpeilen
;; - Beim loslassen des Controllers verspringt das arg
;; - Experte, hat super schnell alles verstanden und gemacht
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
