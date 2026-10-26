' INV: stmt.exit, stmt.function
' SOURCE: spec ms-vbal 5.4.2.18 Exit Function statement
' EXPECT:  1 
Function F() As Long
    F = 1
    Exit Function
    F = 2
End Function
Public Sub Main()
    Debug.Print F()
End Sub
