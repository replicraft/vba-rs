' INV: stmt.stop
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/stop-statement
' EXPECT: a
Public Sub Main()
    Debug.Print "a"
    Stop
    Debug.Print "b"
End Sub
