(ns analysis.bezier)

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
