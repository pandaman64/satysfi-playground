FROM pandaman64/satysfi

WORKDIR /tmp

COPY ./entrypoint.sh .

ENTRYPOINT ["/home/opam/.opam/4.05.0/bin/satysfi", "-o", "output.pdf", "input.saty"]

