' INV: vbal.6.1.3.2.1.2
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Raise: unspecified Description comes from the Error function text, else application-defined message
' EXPECT: d5=Invalid procedure call or argument
' EXPECT: d11=Division by zero
' EXPECT: same=True
' EXPECT: app=Application-defined or object-defined error
' EXPECT: given=custom text
Public Sub Main()
    On Error Resume Next
    Err.Raise 5
    Debug.Print "d5=" & Err.Description
    Err.Clear
    Err.Raise 11
    Debug.Print "d11=" & Err.Description
    Debug.Print "same=" & CStr(Err.Description = Error(11))
    Err.Clear
    Err.Raise 40000
    Debug.Print "app=" & Err.Description
    Err.Clear
    Err.Raise 5, , "custom text"
    Debug.Print "given=" & Err.Description
End Sub
