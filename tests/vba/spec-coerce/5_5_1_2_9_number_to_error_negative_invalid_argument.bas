' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, a Long outside 0..65535 raises Invalid procedure call or argument
' EXPECT: ok
' ERROR: 5
Public Sub Main()
    Dim v As Variant
    v = CVErr(0)
    Debug.Print "ok"
    v = CVErr(-1)
    Debug.Print "not reached"
End Sub
