# custom script to marshal janet file as embedable image
# it will marshal only `main` function instead of entire environment
(do
  (def env (make-env))
  (def entry-env (dofile "test-html.janet" :env env))
  (def- main ((entry-env 'main) :value))
  (def mdict (invert (env-lookup root-env)))
  (def image (marshal main mdict))

  (def out (file/open "out" :wn))
  (file/write out image)
  (file/close out))
  (def chunks (seq [b :in image] (string b)))
  # print raw rust code to directly embed on rust file.
  #
  # (print "static IMAGE_EMBED: &[u8] = &["
  #        (string/join (interpose ", " chunks))
  #        "];"))
