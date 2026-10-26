' INV: stmt.redim, type.variant
' SOURCE: derived assigning an array to a Variant copies it; element assignment converts to the element type and changes only the copy
' EXPECT:  0  5  2 
Public Sub Main()
    Dim a(1 To 2, 1 To 2) As Long
    Dim v As Variant
    v = a
    v(1, 2) = "5"
    Debug.Print a(1, 2); v(1, 2); UBound(v, 2)
End Sub
