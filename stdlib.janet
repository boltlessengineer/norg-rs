# # Interesting issues about macro implementation
#
# - how to "export" markups inside macro? (e.g. quote in `#important` macro, image caption, nested inline markups)

# macro implementation with export-hook pattern

(defn neorg/export/link
  "default link export hook"
  [link lang ctx])

(defn neorg/export/link
  [link lang ctx]
  link)

# TODO: maybe use hiccup syntax: https://github.com/swlkr/janet-html
(defn norg/macro/image
  ".image implementation"
  [meta [src]]
  { :kind :embed
    :export (fn [lang ctx]
      (case lang
        :gfm
          (string
            "!["
            # (neorg/export/inline markup)
            "]("
            (neorg/export/link src :gfm ctx)
            ")\n")
        :html
          (string
            "<img src=\""
            (neorg/export/link src :html-attr ctx)
            "\""
            ">\n"))) })

(defn norg/macro/code
  # TODO: find better way to pass "content" parameter
  [meta [lang] lines]
  { :kind :embed
    :export (fn [lang ctx]
      (case lang
        :gfm
          (string
            "```"
            # TODO: put parameters here
            "\n"
            "```\n")
        :html
          (string
            "<pre"
            # TODO: put parameters here
            "><code>"
            (string/join lines)
            "</code></pre>\n"))) })

(print (let [ao (norg/macro/image nil ["path/to/image.png"])
             export (ao :export)]
         (export :html nil)))

(print (let [ao (norg/macro/code nil ["python"] ["print(\"hello wold\")\n"])
             export (ao :export)]
         (export :html nil)))

# Things neorg should implement
#
# (neorg/export/block [block-node lang ctx])
# (neorg/export/inline [inline-node lang ctx])
# (neorg/export/link [link-object lang ctx])
# (neorg/parse/doc [text])
# (neorg/parse/inline [text])
#
# e.g. (neorg/export/inline :text) can be used to get raw text version of given paragraph
#
# These can be used to share same logic throughout macro implementations

