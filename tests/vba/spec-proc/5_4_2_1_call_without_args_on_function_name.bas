' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call with a bare name uses an empty argument list
' EXPECT: ran
Function F() As Long
    Debug.Print "ran"
End Function
Public Sub Main()
    Call F
End Sub
