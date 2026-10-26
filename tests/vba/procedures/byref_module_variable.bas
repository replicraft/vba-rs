' INV: decl.byref-byval, decl.scope
' SOURCE: spec ms-vbal 5.3.1.11 ByRef binding of a variable argument
' EXPECT:  11 
Dim g As Long
Sub Inc(ByRef n As Long)
    n = n + 1
    g = g + 10
End Sub
Public Sub Main()
    Inc g
    Debug.Print g
End Sub
