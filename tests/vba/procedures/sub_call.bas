' INV: stmt.sub, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call statement
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
