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
(defn norg/tag/image
  ".image implementation"
  [meta [src]]
  [{:kind :embed
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
            `<img src="`
            (neorg/export/link src :html-attr ctx)
            `"`
            ">\n")))}])

(defn norg/tag/code
  # TODO: find better way to pass "lines" parameter
  [meta params lines]
  [{:kind :embed
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
                (create-html-attrs {:class (string "language-" language)}))
              ">"
              (string/join lines)
              "</code></pre>\n"))))}])

# TODO: replace this with macro or sth to call `norg/tag/code` directly
(def norg/tag @{"image" norg/tag/image
                "code"  norg/tag/code})

(defn neorg/export/inline
  [inline lang ctx]
  (case (inline :kind)
    :whitespace " "
    :softbreak " "
    :text (inline :text)
    :special (inline :special)
    :bold (string
      "<strong>"
      ;(map (fn [inline] (neorg/export/inline inline lang ctx)) (inline :markup))
      "</strong>")
    :italic (string
      "<em>"
      ;(map (fn [inline] (neorg/export/inline inline lang ctx)) (inline :markup))
      "</em>")
    :underline (string
      `<span class="underline">`
      ;(map (fn [inline] (neorg/export/inline inline lang ctx)) (inline :markup))
      "</span>")
    :strikethrough (string
      `<span class="strikethrough">`
      ;(map (fn [inline] (neorg/export/inline inline lang ctx)) (inline :markup))
      "</span>")
    :verbatim (string
      "<code>"
      ;(map (fn [inline] (neorg/export/inline inline lang ctx)) (inline :markup))
      "</code>")
    :macro  (let [name   (inline :name)
                  params []
                  markup []
                  ast    ((neorg/inline-tag name) nil params)]
              (string/join (map |(neorg/export/inline $ lang ctx) ast)))
    "TODO_INLINE"))

(defn neorg/export/block
  [block lang ctx]
  (case (block :kind)
    :section (let [heading (block :heading)
                   level (block :level)
                   contents (block :contents)]
               (string
                 "<section>\n"
                 ;(if heading
                   ["<h1>"
                    ;(map |(neorg/export/inline $ lang ctx) heading)
                    "</h1>\n"])
                 ;(map |(neorg/export/block $ lang ctx) contents)
                 "</section>\n"))
    :paragraph (let [inlines (block :inlines)]
                 (string
                   "<p>"
                   ;(map |(neorg/export/inline $ lang ctx) inlines)
                   "</p>\n"))
    :infirm-tag (let [name   (block :name)
                      params (string/split ";" (block :params))
                      ast    ((norg/tag name) nil params)]
                  (string/join (map |(neorg/export/block $ lang ctx) ast)))
    :ranged-tag (let [name   (block :name)
                      params (string/split ";" (block :params))
                      lines  (block :content)
                      ast    ((norg/tag name) nil params lines)]
                  (string/join (map |(neorg/export/block $ lang ctx) ast)))
    :embed ((block :export) lang ctx)
    :unordered-list (let [#params (string/split ";" (block :params))
                          items  (block :items)]
                      (string
                        "<ul>\n"
                        ;(map |(neorg/export/block $ lang ctx) items)
                        "</ul>\n"))
    :ordered-list (let [#params (string/split ";" (block :params))
                        items  (block :items)]
                    (string
                      "<ol>\n"
                      ;(map |(neorg/export/block $ lang ctx) items)
                      "</ol>\n"))
    :quote (let [#params (string/split ";" (block :params))
                 items  (block :items)]
             (string
               "<blockquote>\n"
               ;(map |(string/join (map |(neorg/export/block $ lang ctx) ($ :contents))) items)
               "</blockquote>\n"))
    :list-item (let [#params (string/split ";" (block :params))
                     contents (block :contents)]
                 (string
                   "<li>\n"
                   ;(map |(neorg/export/block $ lang ctx) contents)
                   "</li>\n"))
    "TODO_BLOCK\n"))

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
