' INV: expr.implicit-conv
' SOURCE: derived a numeric string converts when assigned to a numeric variable
' EXPECT:  43 
Public Sub Main()
    Dim n As Long
    n = "42"
    Debug.Print n + 1
End Sub
