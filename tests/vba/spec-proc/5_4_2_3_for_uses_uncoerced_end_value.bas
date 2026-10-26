' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 the algorithm uses the original, uncoerced bound values
' EXPECT: 1,2
Public Sub Main()
    Dim i As Long, s As String
    For i = 1 To 2.5
        If Len(s) > 0 Then s = s & ","
        s = s & CStr(i)
    Next i
    Debug.Print s
End Sub
