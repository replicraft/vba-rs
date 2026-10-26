' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 at least one udt-element must be a udt-member (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Private Type Empty1
    Rem only a comment
End Type
Public Sub Main()
    Debug.Print "ok"
End Sub
