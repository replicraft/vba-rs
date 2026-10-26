' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 negative step ends when the counter is below the end value
' EXPECT: 5,3,1
' EXPECT: neg-none
Public Sub Main()
    Dim i As Long, s As String
    For i = 5 To 1 Step -2
        If Len(s) > 0 Then s = s & ","
        s = s & CStr(i)
    Next i
    Debug.Print s
    For i = 1 To 5 Step -1
        Debug.Print "never"
    Next i
    Debug.Print "neg-none"
End Sub
