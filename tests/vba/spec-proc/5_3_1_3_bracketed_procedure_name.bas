' INV: vbal.5.3.1.3
' SOURCE: derived 5.3.1.3 a procedure name is an identifier; an escaped identifier in brackets may contain spaces and is the same name with and without the brackets
' METHOD: by hand
' EXPECT: n=2
' EXPECT: n=3
Public Sub Main()
    Debug.Print [My Proc](2)
    Debug.Print Module1.[My Proc](3)
End Sub
Private Function [My Proc](n As Long) As String
    [My Proc] = "n=" & n
End Function
