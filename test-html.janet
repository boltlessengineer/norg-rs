(import janet-html :as h)

(def todo-data
  [{:id 1 :value "foo"}
   {:id 2 :value "bar"}
   {:id 3 :value "baz"}])

(defn todo-fmt
  [todo]
  [(todo :id)
   [:strong (todo :value)]])

(defn todo-view
  [todo]
  [:li (todo-fmt todo)])

(defn todos-view
  [data]
  [:ul {:id "foo" :class "bar"}
   (map todo-view data)])

(assert (deep= (h/encode [:div (todos-view todo-data)])
               "<div><ul id=\"foo\" class=\"bar\"><li>1<strong>foo</strong></li><li>2<strong>bar</strong></li><li>3<strong>baz</strong></li></ul></div>"))

(assert (deep= (h/encode [:img {:src "/dog.gif"}])
               "<img src=\"/dog.gif\" />"))

(assert (deep= (h/encode [:br])
               "<br />"))

(assert (deep= (h/encode [:p "Lorem ipsum"])
               "<p>Lorem ipsum</p>"))

(assert (deep= (h/encode [:a {:href "http://github.com"} "GitHub"])
               "<a href=\"http://github.com\">GitHub</a>"))

(assert (deep= (h/encode [:span "Hello " [:em "world!"]])
               "<span>Hello <em>world!</em></span>"))

(defn main [str]
  (pp str)
  (print
    (h/encode [:a {:href str} "boltless.me"]))
  "is this working?")
