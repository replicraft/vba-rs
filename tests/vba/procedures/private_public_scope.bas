' INV: stmt.private, stmt.public, decl.scope
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/private-statement
' EXPECT:  3 
Private p As Long
Public q As Long
Sub Set2()
    p = 1
    q = 2
End Sub
Public Sub Main()
    Set2
    Debug.Print p + q
End Sub
