' INV: vbal.5.4.2.6, stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 a Do without condition clause repeats until Exit Do
' EXPECT: n=5
Public Sub Main()
    Dim n As Long
    Do
        n = n + 1
        If n = 5 Then Exit Do
    Loop
    Debug.Print "n=" & CStr(n)
End Sub
