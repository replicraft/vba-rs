' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 re-entering an already initialised For body continues at the increment step
' EXPECT: 1L23
Public Sub Main()
    Dim i As Long, s As String
    For i = 1 To 3
        If i = 2 Then GoTo Leave
Back:
        s = s & CStr(i)
    Next i
    Debug.Print s
    Exit Sub
Leave:
    s = s & "L"
    GoTo Back
End Sub
