# Copied from https://github.com/swlkr/janet-html
(defn- html/escape [str]
  (let [str (string str)]
    (->> (string/replace-all "&" "&amp;" str)
         (string/replace-all "<" "&lt;")
         (string/replace-all ">" "&gt;")
         (string/replace-all "\"" "&quot;")
         (string/replace-all "'" "&#x27;")
         (string/replace-all "/" "&#x2F;")
         (string/replace-all "%" "&#37;"))))
(defn- html/create-attrs [attrs]
  (reduce
    (fn [acc [attr value]]
      (string acc " " attr `="` value `"`))
    ""
    (map (fn [[x y]] [x y]) (pairs attrs))))

(defn neorg/resolve-linkable
  "link resolver. useful when exporting multiple files
   it can also be used to style links based on its target"
  [linkable]
  # expect linkable to be a link or an anchor
  linkable)

(defn norg/parse/doc
  "parse document"
  [text]
  (cond
    (string? text) (error "todo")
    (array? text) (error "todo")
    (tuple? text) (error "todo")))

(defn norg/parse/inline
  "parse inline text"
  [text]
  (error "todo"))

(defn norg/tag/image
  ".image implementation"
  [[src]]
  [{:kind :embed
    :export (fn [lang ctx]
              (case lang
                :gfm (string
                       "!["
                       # (neorg/export/inline (neorg/parse/inline alt))
                       "]("
                       src
                       ")\n")
                :html (string
                        "<img"
                        (html/create-attrs {:src src})
                        ">\n")))}])

(defn norg/tag/code
  # TODO: find better way to pass "lines" parameter
  [params lines]
  [{:kind :embed
    :export (fn [target ctx]
              (case target
                # TODO: find if there is a line starting with three or more backticks
                :gfm (string
                       "```"
                       (string/join params " ")
                       "\n"
                       (string/join lines)
                       "```\n")
                :html (let [language (get params 0)]
                        (string
                          "<pre><code"
                          (if language
                            (html/create-attrs {:class (string "language-" language)}))
                          ">"
                          (string/join lines)
                          "</code></pre>\n"))))}])
    # :export {:gfm (fn [ctx]
    #                 (string
    #                   "```" (string/join params " ")
    #                   "\n"
    #                   (string/join lines)
    #                   "```\n"))
    #          :html (fn [ctx]
    #                  (let [language (get params 0)]
    #                    (string
    #                      "<pre><code"
    #                      (if language
    #                        (html/create-attrs {:class (string "language-" language)}))
    #                      ">"
    #                      (string/join lines)
    #                      "</code></pre>\n")))}}])

# TODO: replace this with macro or sth to call `norg/tag/code` directly
(def norg/tag @{"image" norg/tag/image
                "code" norg/tag/code})

(defn norg/inline-tag/img
  [[src] markup]
  [{:kind :embed
    :export (fn [target ctx]
              (case target
                :html (string
                        "<img"
                        (html/create-attrs {:src src})
                        ">")))}])

(def norg/inline-tag @{"img" norg/inline-tag/img})

(defn neorg/export/inline
  "export norg inline node"
  [inline lang ctx]
  (case (inline :kind)
    :whitespace " "
    :softbreak "\n"
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
    :macro (let [name (inline :name)
                 params (inline :attrs)
                 markup []
                 ast ((norg/inline-tag name) params markup)]
             (string/join (map |(neorg/export/inline $ lang ctx) ast)))
    :link (let [href (inline :target)]
            (string
              "<a"
              (html/create-attrs {:href href})
              ">"
              ;(map |(neorg/export/inline $ lang ctx) (inline :markup))
              "</a>"))
    :anchor "todo-anchor"
    :embed ((inline :export) lang ctx)
    "TODO_INLINE"))

(defn neorg/export/block
  "export norg block node"
  [block lang & ctx]
  (default ctx @{})
  (case (block :kind)
    :section (let [heading (block :heading)
                   level (block :level)
                   level (if (> level 6) 6 level)
                   contents (block :contents)]
               (string
                 "<section>\n"
                 ;(if heading
                    ["<h" level ">"
                     ;(map |(neorg/export/inline $ lang ctx) heading)
                     "</h" level ">\n"])
                 ;(map |(neorg/export/block $ lang ctx) contents)
                 "</section>\n"))
    :paragraph (let [inlines (block :inlines)]
                 (string
                   "<p>"
                   ;(map |(neorg/export/inline $ lang ctx) inlines)
                   "</p>\n"))
    :infirm-tag (let [name (block :name)
                      params (string/split ";" (block :params))
                      ast ((norg/tag name) params)]
                  (string/join (map |(neorg/export/block $ lang ctx) ast)))
    :ranged-tag (let [name (block :name)
                      params (string/split ";" (block :params))
                      lines (block :content)
                      ast ((norg/tag name) params lines)]
                  (string/join (map |(neorg/export/block $ lang ctx) ast)))
    :embed ((block :export) lang ctx)
    :unordered-list (let [#params (string/split ";" (block :params))
                          items (block :items)]
                      (string
                        "<ul>\n"
                        ;(map |(neorg/export/block $ lang ctx) items)
                        "</ul>\n"))
    :ordered-list (let [#params (string/split ";" (block :params))
                        items (block :items)]
                    (string
                      "<ol>\n"
                      ;(map |(neorg/export/block $ lang ctx) items)
                      "</ol>\n"))
    :quote (let [#params (string/split ";" (block :params))
                 items (block :items)]
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

(defn neorg/export
  "export list of blocks"
  [blocks lang & ctx]
  (default ctx @{})
  (string/join
    (map |(neorg/export/block $ lang)
         blocks)))
