' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, at run time an array value indexed with the wrong number of arguments raises error 9
' ERROR: 9
Public Sub Main()
    Dim a(1 To 2, 1 To 2) As Long
    Dim v As Variant
    Dim x As Variant
    v = a
    x = v(1)
    Debug.Print "not reached"
End Sub
