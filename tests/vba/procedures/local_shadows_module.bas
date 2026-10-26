' INV: decl.scope
' SOURCE: spec ms-vbal 5.6.10 simple name resolution of module and procedure variables
' EXPECT:  5  1 
Dim x As Long
Public Sub Main()
    x = 5
    Dim x2 As Long
    x2 = 1
    Debug.Print x; x2
End Sub
