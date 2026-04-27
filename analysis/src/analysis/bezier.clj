(ns analysis.bezier
  (:require [clojure.math :as math]))

(defn cart
  "Build the cartesian product of the input lists"
  ([xs]
   xs)
  ([xs ys]
   (mapcat (fn [x] (map (fn [y] (list x y)) ys)) xs))
  ([xs ys & more]
   (mapcat (fn [x] (map (fn [z] (cons x z)) (apply cart (cons ys more)))) xs)))

(defn lerp
  "lerp two 3d vectors by t"
  [a b t]
  (vector (+ (* (a 0) (- 1 t))
             (* (b 0) t))
          (+ (* (a 1) (- 1 t))
             (* (b 1) t))
          (+ (* (a 2) (- 1 t))
             (* (b 2) t))))

(defn dot
  "Compute the dot product of two vectors.
  When the length is not equal, throw an error"
  [a b]
  (if (not (= (count a) (count b)))
    (throw (ex-message "cannot dot product different sized vectors"))
    (reduce + 0 (map (fn [a b] (* a b)) a b))))

(defn add
  "Compute the dot product of two vectors.
  When the length is not equal, throw an error"
  [a b]
  (if (not (= (count a) (count b)))
    (throw (ex-message "cannot dot product different sized vectors"))
    (mapv (fn [a b] (+ a b)) a b)))

(defn sub
  "Compute the dot product of two vectors.
  When the length is not equal, throw an error"
  [a b]
  (if (not (= (count a) (count b)))
    (throw (ex-message "cannot dot product different sized vectors"))
    (mapv (fn [a b] (- a b)) a b)))

(defn magnitude
  "Compute the magnitude / length of a vector"
  [v]
  (math/sqrt (dot v v)))

(defn- greedy-min-selection
  "Select the min distanced point from a to bs by returning the index or -1, while respecting the ignore list"
  [a bs ignore-set]
  (loop [[b & bs] bs
         b-idx 0
         b-min-idx -1
         min-dist Double/MAX_VALUE]

    (if b
      (if (and (not (get ignore-set b-idx)) (< (magnitude (sub a b)) min-dist))
        (recur bs (+ b-idx 1) b-idx (magnitude (sub a b)))
        (recur bs (+ b-idx 1) b-min-idx min-dist))
      (list b-min-idx min-dist))))

(defn greedy-min-assignment
  "greedily assign the points of list bs to indices of list as, by selection the min distance for each point, greedily.
  Since this is a greedy algorithm, this is not the local optima"
  [as bs]
  (if (> (count as) (count bs))
    (throw (Exception. "can't assign each a value to a b value"))
    (loop [[a & as] as
           a-idx 0
           assignments {}
           ignore-set #{}]
      (if a
        (let [[bs-min-idx bs-min-dist] (greedy-min-selection a bs ignore-set)]
          (if (= bs-min-idx -1)
            (throw (Exception. "can't assign a idx to -1"))
            (recur as
                   (+ a-idx 1)
                   (assoc assignments a-idx {:idx bs-min-idx
                                             :dist bs-min-dist})
                   (conj ignore-set bs-min-idx))))
        assignments))))

(defn decas
  "Compute the decasteljau result for points and parameter t
  Expects points to be a non lazy vector (not list!)"
  [points t]
  (let [n (- (count points) 1)]
    (loop [ps points
           n n]
      (if (> n 0)
        (recur (loop [i 0
                      pts ps]
                 (if (< i n)
                   (recur (+ i 1)
                          (assoc pts
                                 i
                                 (lerp (pts i) (pts (+ i 1)) t)))
                   pts))
               (- n 1))
        (first ps)))))

(defn decas-2d
  [rows u v]
  (-> (for [row rows
            :let [col (decas row u)]]
        col)
      (vec)
      (decas v)))

(decas (vector [0 6 0] [6 8 0] [8 4 0] [2 0 0]) 0.5)
(decas-2d [[[0 6 0] [6 8 0] [8 4 0] [2 0 0]]
           [[0 6 1] [6 8 1] [8 4 1] [2 0 1]]
           [[0 6 2] [6 8 2] [8 4 2] [2 0 2]]] 0.5 0.5)
