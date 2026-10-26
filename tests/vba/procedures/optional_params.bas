' INV: decl.optional
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT: 1,10,z
' EXPECT: 1,2,z
' EXPECT: 1,10,q
Function F(a As Long, Optional b As Long = 10, Optional c As String = "z") As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(1)
    Debug.Print F(1, 2)
    Debug.Print F(1, , "q")
End Sub
