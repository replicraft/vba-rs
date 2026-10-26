' INV: decl.byref-byval, decl.array-bounds
' SOURCE: spec ms-vbal 5.3.1.11 ByRef binding of a variable argument, 2.3.1 dependent variables
' EXPECT:  0  2 
Sub Inc(ByRef n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim a(1 To 2) As Long
    Inc a(2)
    Inc a(2)
    Debug.Print a(1); a(2)
End Sub
