' INV: stmt.sub, stmt.call
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/call-statement
' EXPECT: hi a
' EXPECT: hi b
' EXPECT: hi c
Sub Hello(ByVal n As String)
    Debug.Print "hi " & n
End Sub
Public Sub Main()
    Call Hello("a")
    Hello "b"
    Hello ("c")
End Sub
