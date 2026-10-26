' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, a Variant holding a non-class non-array value with an argument list raises error 9
' ERROR: 9
Public Sub Main()
    Dim v As Variant
    Dim x As Variant
    v = 5
    x = v(1)
    Debug.Print "not reached"
End Sub
