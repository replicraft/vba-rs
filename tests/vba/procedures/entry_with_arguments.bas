' INV: stmt.function
' SOURCE: derived procedures with parameters can be run by name through the facade
' EXPECT:  3 
Public Sub Main(Optional ByVal x As Long = 3)
    Debug.Print x
End Sub
