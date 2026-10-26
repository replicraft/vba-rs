' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, trailing non-numeric text is not allowed by the grammar
' ERROR: 13
Public Sub Main()
    Dim l As Long
    l = "12abc"
    Debug.Print "not reached"
End Sub
