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

# (defn neorg/resolve-linkable
#   "link resolver. useful when exporting multiple files
#    it can also be used to style links based on its target"
#   [linkable]
#   # expect linkable to be a link or an anchor
#   linkable)

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

(defn- norg/tag/image
  ".image implementation"
  [[src]]
  [{:kind :embed
    :export {:gfm (fn [ctx]
                    (string
                      "!["
                      # (neorg/export/inline (neorg/parse/inline alt))
                      "]("
                      src
                      ")\n"))
             :html (fn [ctx]
                     (string
                       "<img"
                       (html/create-attrs {:src src})
                       ">\n"))}}])

(defn- norg/tag/code
  # TODO: find better way to pass "lines" parameter
  [params lines]
  [{:kind :embed
    :export {:gfm (fn [ctx]
                    # TODO: find if there is a line starting with three or more backticks
                    (string
                      "```" (string/join params " ")
                      "\n"
                      (string/join lines)
                      "```\n"))
             :html (fn [ctx]
                     (let [language (get params 0)]
                       (string
                         "<pre><code"
                         (if language
                           (html/create-attrs {:class (string "language-" language)}))
                         ">"
                         # TODO: find if there is a line starting with three or more backticks
                         ;(map html/escape lines)
                         "</code></pre>\n")))}}])

(def message "message from neorg environment")

(defn- norg/tag/eval
  [params lines]
  (defn chunk-string [lines]
    (def lines (reverse lines))
    (fn [buf _]
      (when-let [line (array/pop lines)]
        (buffer/push buf line))))
  # evaluate given lines
  (run-context
    {:env (curenv)
     :chunks (chunk-string lines)})
  [])

(defn- norg/inline-tag/img
  [[src] markup]
  [{:kind :embed
    :export {:html (fn [ctx]
                     (string
                       "<img"
                       (html/create-attrs {:src src})
                       ">"))}}])

# tables where Neorg can register dynamically
(def norg/ast/tag
  "name tag with \\ prefix to add as inline tag"
  @{"image" norg/tag/image
    "code" norg/tag/code
    "eval" norg/tag/eval
    "\\img" norg/inline-tag/img})

(def norg/export-hook
  "key is tuple of [:target :kind]"
  @{})

(defn norg/export/inline
  [lang inline ctx]
  (def hook (norg/export-hook [lang (inline :kind)]))
  (if hook
    (hook inline ctx)
    (case lang
      :html (case (inline :kind)
              :whitespace " "
              :softbreak "\n"
              :text (inline :text)
              :special (html/escape (inline :special))
              :bold (string
                      "<strong>"
                      ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                      "</strong>")
              :italic (string
                        "<em>"
                        ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                        "</em>")
              :underline (string
                           `<span class="underline">`
                           ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                           "</span>")
              :strikethrough (string
                               `<span class="strikethrough">`
                               ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                               "</span>")
              :verbatim (string
                          "<code>"
                          ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                          "</code>")
              :macro (let [name (inline :name)
                           params (inline :attrs)
                           markup []
                           tag (norg/ast/tag (string "\\" name))]
                       (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                       (def ast (tag params markup))
                       (string/join (map |(norg/export/inline lang $ ctx) ast)))
              :link (let [href (inline :target)]
                      (string
                        "<a"
                        (html/create-attrs {:href href})
                        ">"
                        ;(map |(norg/export/inline lang $ ctx) (inline :markup))
                        "</a>"))
              :anchor "todo-anchor"
              :embed (((inline :export) lang) ctx)
              "TODO_INLINE"))))

(defn norg/export/block
  [lang block ctx]
  (if-let [hook (norg/export-hook [lang (block :kind)])]
    (hook block ctx)
    (case lang
      :html (case (block :kind)
              :section (let [heading (block :heading)
                             level (block :level)
                             level (if (> level 6) 6 level)
                             contents (block :contents)]
                         (string
                           "<section>\n"
                           ;(if heading
                              ["<h" level ">"
                               ;(map |(norg/export/inline lang $ ctx) heading)
                               "</h" level ">\n"])
                           ;(map |(norg/export/block lang $ ctx) contents)
                           "</section>\n"))
              :paragraph (let [inlines (block :inlines)]
                           (string
                             "<p>"
                             ;(map |(norg/export/inline lang $ ctx) inlines)
                             "</p>\n"))
              :infirm-tag (let [name (block :name)
                                params (string/split ";" (block :params))
                                tag (norg/ast/tag name)]
                            (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                            (def ast (tag params))
                            (string/join (map |(norg/export/block lang $ ctx) ast)))
              :ranged-tag (let [name (block :name)
                                params (string/split ";" (block :params))
                                lines (block :content)
                                tag (norg/ast/tag name)]
                            (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                            (def ast (tag params lines))
                            (string/join (map |(norg/export/block lang $ ctx) ast)))
              :embed (((block :export) lang) ctx)
              :unordered-list (let [#params (string/split ";" (block :params))
                                    items (block :items)]
                                (string
                                  "<ul>\n"
                                  ;(map |(norg/export/block lang $ ctx) items)
                                  "</ul>\n"))
              :ordered-list (let [#params (string/split ";" (block :params))
                                  items (block :items)]
                              (string
                                "<ol>\n"
                                ;(map |(norg/export/block lang $ ctx) items)
                                "</ol>\n"))
              :quote (let [#params (string/split ";" (block :params))
                           items (block :items)]
                       (string
                         "<blockquote>\n"
                         ;(map |(string/join
                                  (map |(norg/export/block lang $ ctx) ($ :contents)))
                               items)
                         "</blockquote>\n"))
              :list-item (let [#params (string/split ";" (block :params))
                               contents (block :contents)]
                           (string
                             "<li>\n"
                             ;(map |(norg/export/block lang $ ctx) contents)
                             "</li>\n"))
              "TODO_BLOCK\n"))))

(defn norg/export/doc
  [lang ast & ctx]
  (default ctx @{})
  (string/join (map |(norg/export/block lang $ ctx)
                    ast)))
