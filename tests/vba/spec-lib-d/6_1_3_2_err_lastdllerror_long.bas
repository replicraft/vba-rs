' INV: vbal.6.1.3.2.2.4
' SOURCE: spec ms-vbal 6.1.3.2.2.4 LastDllError is a Long property, not changed (and no error raised) by non-DLL activity
' EXPECT: t=Long
' EXPECT: same=True
Public Sub Main()
    Dim before As Long
    before = Err.LastDllError
    On Error Resume Next
    Err.Raise 5
    Debug.Print "t=" & TypeName(Err.LastDllError)
    Debug.Print "same=" & CStr(Err.LastDllError = before)
End Sub
