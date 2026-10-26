' INV: vbal.5.6.9.3
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators runtime semantics, an Error operand combined with a non-Error value raises Type mismatch
' XFAIL: wrong pending
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim x As Variant
    v = CVErr(5)
    x = v + 1
    Debug.Print "not reached"
End Sub
