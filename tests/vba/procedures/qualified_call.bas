' INV: stmt.sub, stmt.function
' SOURCE: derived Module.Proc qualification
' EXPECT:  7 
Public Function F() As Long
    F = 7
End Function
Public Sub Main()
    Debug.Print Module1.F()
End Sub
