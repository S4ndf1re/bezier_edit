(defproject analysis "0.1.0-SNAPSHOT"
  :description "FIXME: write description"
  :url "https://example.com/FIXME"
  :license {:name "EPL-2.0 OR GPL-2.0-or-later WITH Classpath-exception-2.0"
            :url "https://www.eclipse.org/legal/epl-2.0/"}
  :dependencies [[org.clojure/clojure "1.12.2"]
                 [io.github.nextjournal/clerk "0.18.1158"]
                 [meta-csv "0.1.0"]
                 [scicloj/tablecloth "8.016"]
                 [cheshire "6.2.0"]
                 [generateme/fastmath "2.4.0"]]
  :main ^:skip-aot analysis.core
  :target-path "target/%s"
  :profiles {:uberjar {:aot :all
                       :jvm-opts ["-Dclojure.compiler.direct-linking=true"]}})
