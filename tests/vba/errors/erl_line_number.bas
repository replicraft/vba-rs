' INV: stmt.on-error, lex.label
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/err-object
' EXPECT: a
' EXPECT:  20 
Public Sub Main()
    On Error GoTo H
    10 Debug.Print "a"
    20 Error 5
    Exit Sub
H:
    Debug.Print Erl
End Sub
