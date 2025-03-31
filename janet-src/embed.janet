(use "./stdlib")

(put norg/ast/tag
     "\\gh"
     (fn [[src] markup]
         [{:kind :link
           :target (string "https://github.com/" src)
           :markup [{:kind :text
                     :text src}]}]))

# test custom export hook for standard block node.
(put norg/export-hook
     [:html :paragraph]
     (fn [block ctx]
       (let [inlines (block :inlines)]
         (string
           `<p class="paragraph">`
           ;(map |(norg/export/inline :html $ ctx) inlines)
           `</p>\n`))))

(defn main [lang ast &]
  (norg/export/doc lang ast))
