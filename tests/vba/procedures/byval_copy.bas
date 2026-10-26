' INV: decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByVal parameter is a local copy
' EXPECT:  1 
Sub Inc(ByVal n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim x As Long
    x = 1
    Inc x
    Debug.Print x
End Sub
