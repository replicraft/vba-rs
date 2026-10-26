' INV: decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.5 parameters are ByRef by default, 5.3.1.11 ByRef binding
' EXPECT:  2 
Sub Inc(n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim x As Long
    x = 1
    Inc x
    Debug.Print x
End Sub
