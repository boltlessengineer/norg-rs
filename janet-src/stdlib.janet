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

# (defn norg/parse/doc
#   "parse document"
#   [text]
#   (cond
#     (string? text) (error "todo")
#     (array? text) (error "todo")
#     (tuple? text) (error "todo")))

# (defn norg/parse/inline
#   "parse inline text"
#   [text]
#   (error "todo"))

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

(defn- parse-attrs [attr-list]
  (def attrs @{})
  (if attr-list
    (each attr attr-list
      (match attr
        [key value] (put attrs key value)
        [key] (put attrs key true))))
  attrs)

(defn- filter-attrs [lang attrs]
  (def prefix (string lang "."))
  (def filtered @{})
  (loop [[key value] :pairs attrs]
    (if (string/has-prefix? prefix key)
      (put filtered (string/slice key 5) value)))
  filtered)

(defmacro norg/export/inline-html-impl []
  '(do
     (defn- merge-html-attrs
         [& tables]
         (def merged @{})
         (each tbl tables
           (if tbl
             (loop [[k v] :pairs tbl]
               (def k (keyword k))
               (if (string? (merged k))
                 (put merged k (string (merged k) " " v))
                 (put merged k v)))))
         merged)
     (defn attached-modifier
       [tag &opt attrs]
       (string
         "<" tag
         (html/create-attrs
           (merge-html-attrs attrs (filter-attrs :html (parse-attrs (inline :attrs)))))
         ">"
         ;(map |(norg/export/inline :html $ ctx) (inline :markup))
         "</" tag ">"))
     (case (inline :kind)
       :whitespace " "
       :softbreak "\n"
       :text (html/escape (inline :text))
       :special (html/escape (inline :special))
       :bold (attached-modifier :strong)
       :italic (attached-modifier :em)
       :underline (attached-modifier :span {:class "underline"})
       :strikethrough (attached-modifier :span {:class "strikethrough"})
       :verbatim (attached-modifier :code)
       :link (let [href (inline :target)
                   markup (inline :markup)
                   attrs {:href href}]
               (string
                 "<a"
                 (html/create-attrs
                   (merge-html-attrs attrs (filter-attrs :html (parse-attrs (inline :attrs)))))
                 ">"
                 (if markup
                   (string/join (map |(norg/export/inline :html $ ctx) markup))
                   (html/escape href))
                 "</a>"))
       :anchor "TODO_ANCHOR"
       "TODO_INLINE")))

(defn norg/export/inline
  [lang inline ctx]
  (def hook (norg/export-hook [lang (inline :kind)]))
  (cond
    hook (hook inline ctx)
    (= (inline :kind) :embed) (((inline :export) :html) ctx)
    (= (inline :kind) :macro) (let [name (inline :name)
                                    params (parse-attrs (inline :attrs))
                                    markup []
                                    tag (norg/ast/tag (string "\\" name))]
                                (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                                (def ast (tag params markup))
                                (string/join (map |(norg/export/inline lang $ ctx) ast)))
    (case lang
      :html (norg/export/inline-html-impl)
      # :gfm (norg/export/inline-gfm-impl)
      (error "unkown language"))))

(defmacro- norg/export/block-html-impl []
  '(case (block :kind)
     :section (let [heading (block :heading)
                    level (block :level)
                    level (if (> level 6) 6 level)
                    contents (block :contents)]
               (string
                "<section>\n"
                ;(if heading
                   ["<h" level ">"
                     ;(map |(norg/export/inline :html $ ctx) heading)
                     "</h" level ">\n"])
                 ;(map |(norg/export/block lang $ ctx) contents)
                 "</section>\n"))
     :paragraph (let [inlines (block :inlines)]
                 (string
                   "<p>"
                   ;(map |(norg/export/inline :html $ ctx) inlines)
                   "</p>\n"))
     :unordered-list (let [#params (string/split ";" (block :params))
                           items (block :items)]
                       (string
                         "<ul>\n"
                         ;(map |(norg/export/block :html $ ctx) items)
                         "</ul>\n"))
     :ordered-list (let [#params (string/split ";" (block :params))
                         items (block :items)]
                     (string
                       "<ol>\n"
                       ;(map |(norg/export/block :html $ ctx) items)
                       "</ol>\n"))
     :quote (let [#params (string/split ";" (block :params))
                  items (block :items)]
             (string
              "<blockquote>\n"
              ;(map |(string/join
                      (map |(norg/export/block :html $ ctx) ($ :contents)))
                    items)
              "</blockquote>\n"))
     :list-item (let [#params (string/split ";" (block :params))
                      contents (block :contents)]
                 (string
                   "<li>\n"
                   ;(map |(norg/export/block :html $ ctx) contents)
                   "</li>\n"))
     "TODO_BLOCK\n"))

(defn norg/export/block
  [lang block ctx]
  (def hook (norg/export-hook [lang (block :kind)]))
  (cond
    hook (hook block ctx)
    (= (block :kind) :embed) (((block :export) :html) ctx)
    (= (block :kind) :infirm-tag) (let [name (block :name)
                                        params (string/split ";" (block :params))
                                        tag (norg/ast/tag name)]
                                    (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                                    (def ast (tag params))
                                    (string/join (map |(norg/export/block lang $ ctx) ast)))
    (= (block :kind) :ranged-tag) (let [name (block :name)
                                        params (string/split ";" (block :params))
                                        lines (block :content)
                                        tag (norg/ast/tag name)]
                                    (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                                    (def ast (tag params lines))
                                    (string/join (map |(norg/export/block lang $ ctx) ast)))
    (case lang
      :html (norg/export/block-html-impl)
      (error "unkown language"))))

(defn norg/export/doc
  [lang ast & ctx]
  (default ctx @{})
  (string/join (map |(norg/export/block lang $ ctx)
                    ast)))
