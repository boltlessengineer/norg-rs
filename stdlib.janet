# # Interesting issues about macro implementation
#
# - how to "export" markups inside macro? (e.g. quote in `#important` macro, image caption, nested inline markups)

# macro implementation with export-hook pattern

(defn neorg/export/link
  "default link export hook"
  [link lang ctx]
  link)

(defn create-html-attrs
  [attrs]
  (reduce
    (fn [acc [attr value]]
      (string acc " " attr `="` value `"`))
    ""
    (map (fn [[x y]] [x y]) (pairs attrs))))

# TODO: maybe use hiccup syntax: https://github.com/swlkr/janet-html
# TODO: rewrite to single `norg/tag` table that holds every macros
(defn norg/tag/image
  ".image implementation"
  [meta [src]]
  { :kind :embed
    :export (fn [lang ctx]
      (case lang
        :gfm
          (string
            "!["
            # (neorg/export/inline (neorg/parse/doc alt))
            "]("
            (neorg/export/link src :gfm ctx)
            ")\n")
        :html
          (string
            "<img src=\""
            (neorg/export/link src :html-attr ctx)
            "\""
            ">\n"))) })

(defn norg/tag/code
  # TODO: find better way to pass "content" parameter
  [meta params lines]
  { :kind :embed
    :export (fn [target ctx]
      (case target
        :gfm
          # TODO: find if there is a line starting with three or more backticks
          (string
            "```"
            (string/join params " ")
            "\n"
            (string/join lines)
            "```\n")
        :html
          (let [language (get params 0)]
            (string
              "<pre><code"
              (if language
                (create-html-attrs { :class (string "language-" language) }))
              ">"
              (string/join lines)
              "</code></pre>\n")))) })

# (print (let [ao (norg/tag/image nil ["path/to/image.png"])
#              export (ao :export)]
#          (export :html nil)))

# (print (let [ao (norg/tag/code nil ["python"] ["print(\"hello wold\")\n"])
#              export (ao :export)]
#          (export :html nil)))

# Things neorg should implement
#
# (neorg/export/block [block lang ctx])
# (neorg/export/inline [inline lang ctx])
(defn _neorg/parse/doc
  "parse document"
  [text]
  (cond
    (string? text) (error "todo")
    (array? text)  (error "todo")
    (tuple? text)  (error "todo")))
(defn _neorg/parse/inline
  "parse inline text"
  [text]
  (print "todo"))
# e.g. (neorg/export/inline inline :text ctx) can be used to get raw text version of given paragraph
#
# These can be used to share same export logic throughout macro implementations
