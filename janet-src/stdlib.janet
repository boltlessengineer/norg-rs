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

(defn neorg/query-documents
  "query norg documents"
  [query]
  (error "neorg/query-documents is not yet implemented"))

(defn neorg/resolve-link-target
  "link resolver. returns absolute path to the note based on given link target"
  [target]
  # TODO: parse `target` here..?
  (error "neorg/resolve-link-target is not yet implemented"))

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

# (defn norg/parse/link
#   "parse link target"
#   [link]
#   (error "todo"))

(defn- handle-atom
  [atom]
  (def atom (string/trim atom))
  (cond
    (= atom "nil") :nil
    (= atom "true") true
    (= atom "false") false
    (if-let [num (scan-number atom)]
      num
      atom)))
(def- special "{}[]:\n")
(def meta-peg
  (peg/compile
    ~{:main (* (any :property) -1)
      :value (+ :array :object :atom)
      :eol (+ "\r\n" "\r" "\n")
      :space* (any (set " \t"))
      :array (/ (* "["
                   :s*
                   (any (+ :value
                           (* :eol :space*)))
                   "]")
                ,|[;$&])
      :key (<- (some (if-not (set "{}[]:\n") 1)))
      :property (* :space*
                   :key
                   ":"
                   :space*
                   (+ :value
                      (constant :nil))
                   :s*)
      :object (/ (* "{"
                    :s*
                    (any (+ :property
                            (* :eol :space*)))
                    "}")
                 ,|(struct ;$&))
      :atom (/ (<- (some (if-not (set "{}[]\n") 1)))
               ,handle-atom)}))

(defn norg/meta/parse
  [text]
  (struct ;(peg/match meta-peg text)))

(defn- norg/tag/image
  ".image implementation"
  [ctx [src]]
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
                       "<figure><img"
                       (html/create-attrs {:src src})
                       "></figure>\n"))}}])

(defn- norg/tag/code
  # TODO: find better way to pass "lines" parameter
  [ctx params lines]
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
                         "<figure><pre><code"
                         (if language
                           (html/create-attrs {:class (string "language-" language)}))
                         ">"
                         # TODO: find if there is a line starting with three or more backticks
                         ;(map html/escape lines)
                         "</code></pre></figure>\n")))}}])

(defn- norg/tag/tada
  [ctx params]
  [{:kind :paragraph
    :inlines [{:kind :text
               :text "tada"}]}])

(def message "message from neorg environment")

(defn- norg/tag/eval
  [ctx params lines]
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

(defn- norg/tag/document.meta
  [ctx params lines]
  (def text (string/join lines))
  (def meta (norg/meta/parse text))
  (put ctx :meta meta)
  [])

(defn- norg/inline-tag/img
  [ctx [src] markup]
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
    "document.meta" norg/tag/document.meta
    "tada" norg/tag/tada
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
       :hardbreak "<br>"
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
                                (def ast (tag ctx params markup))
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
                "<section"
                (html/create-attrs
                  (filter-attrs :html (parse-attrs (block :attrs))))
                ">\n"
                ;(if heading
                   ["<h" level
                    ">"
                    ;(map |(norg/export/inline :html $ ctx) heading)
                    "</h" level ">\n"])
                 ;(map |(norg/export/block lang $ ctx) contents)
                 "</section>\n"))
     :paragraph (let [inlines (block :inlines)]
                 (string
                   "<p"
                   (html/create-attrs
                     (filter-attrs :html (parse-attrs (block :attrs))))
                   ">"
                   ;(map |(norg/export/inline :html $ ctx) inlines)
                   "</p>\n"))
     :unordered-list (let [items (block :items)]
                       (string
                         "<ul"
                         (html/create-attrs
                            (filter-attrs :html (parse-attrs (block :attrs))))
                         ">\n"
                         ;(map |(norg/export/block :html $ ctx) items)
                         "</ul>\n"))
     :ordered-list (let [items (block :items)]
                     (string
                       "<ol"
                       (html/create-attrs
                         (filter-attrs :html (parse-attrs (block :attrs))))
                       ">\n"
                       ;(map |(norg/export/block :html $ ctx) items)
                       "</ol>\n"))
     :quote (let [items (block :items)]
             (string
              "<blockquote"
              (html/create-attrs
                (filter-attrs :html (parse-attrs (block :attrs))))
              ">\n"
              ;(map |(string/join
                      (map |(norg/export/block :html $ ctx) ($ :contents)))
                    items)
              "</blockquote>\n"))
     :list-item (let [contents (block :contents)]
                 (string
                   "<li"
                   (html/create-attrs
                     (filter-attrs :html (parse-attrs (block :attrs))))
                   ">\n"
                   ;(map |(norg/export/block :html $ ctx) contents)
                   "</li>\n"))
     :horizontal-line (string
                        "<hr"
                        (html/create-attrs
                          (filter-attrs :html (parse-attrs (block :attrs))))
                        ">\n")
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
                                    (def ast (tag ctx params))
                                    (string/join (map |(norg/export/block lang $ ctx) ast)))
    (= (block :kind) :ranged-tag) (let [name (block :name)
                                        # HACK: stupid. should parse these from tree-sitter parser
                                        params (block :params)
                                        params (if params params "")
                                        params (string/split ";" params)
                                        lines (block :content)
                                        tag (norg/ast/tag name)]
                                    (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                                    (def ast (tag ctx params lines))
                                    (string/join (map |(norg/export/block lang $ ctx) ast)))
    (= (block :kind) :carryover-tag) (let [name (block :name)
                                           params (block :params)
                                           params (if params params "")
                                           params (string/split ";" params)
                                           target (block :target)
                                           tag (norg/ast/tag name)]
                                       (unless (truthy? tag) (error (string "tag '" name "' doesn't exist")))
                                       (def ast (tag ctx params target))
                                       (string/join (map |(norg/export/block lang $ ctx) ast)))
    (case lang
      :html (norg/export/block-html-impl)
      (error "unkown language"))))

(defn norg/export/doc
  [lang ast &opt ctx]
  (default ctx @{:meta @{}})
  (def res (string/join (map |(norg/export/block lang $ ctx)
                           ast)))
  [res ctx])
