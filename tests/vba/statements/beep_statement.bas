' INV: stmt.beep
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/beep-statement
' EXPECT: ok
Public Sub Main()
    Beep
    Debug.Print "ok"
End Sub
