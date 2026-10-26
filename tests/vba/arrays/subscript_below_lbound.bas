' INV: decl.array-bounds
' SOURCE: spec ms-vbal 5.6.2.1 array element read outside its dimension raises error 9
' ERROR: 9
Public Sub Main()
    Dim a(1 To 3) As Long
    Debug.Print a(0)
End Sub
