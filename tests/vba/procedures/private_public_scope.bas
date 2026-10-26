' INV: stmt.private, stmt.public, decl.scope
' SOURCE: spec ms-vbal 5.2.3.1.1 module variable declarations, 5.6.10 simple name expressions
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
