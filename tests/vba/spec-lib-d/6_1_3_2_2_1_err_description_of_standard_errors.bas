' INV: vbal.6.1.3.2.2.1, obj.err
' SOURCE: derived ms-vbal 6.1.3.2.2.1 Err.Description of a raised error holds the standard message for its number, or the text given to Raise
' EXPECT: 5 Invalid procedure call or argument
' EXPECT: 11 Division by zero
' EXPECT: 13 Type mismatch
' EXPECT: 9 Subscript out of range
' EXPECT: custom text
' EXPECT: Application-defined or object-defined error
' EXPECT: set by user
Private Sub Show(ByVal n As Long)
    On Error Resume Next
    Err.Raise n
    Debug.Print CStr(Err.Number) & " " & Err.Description
    Err.Clear
End Sub
Public Sub Main()
    Show 5
    Show 11
    Show 13
    Show 9
    On Error Resume Next
    Err.Raise 5, "src", "custom text"
    Debug.Print Err.Description
    Err.Clear
    Err.Raise 1234
    Debug.Print Err.Description
    Err.Clear
    Err.Description = "set by user"
    Debug.Print Err.Description
End Sub
