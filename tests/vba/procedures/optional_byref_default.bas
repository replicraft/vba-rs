' INV: decl.optional
' SOURCE: spec ms-vbal 5.3.1.5 optional parameter default value, 5.3.1.11 unmapped optional parameters
' EXPECT:  6  11 
Function F(Optional ByVal n As Long = 5) As Long
    n = n + 1
    F = n
End Function
Public Sub Main()
    Debug.Print F(); F(10)
End Sub
