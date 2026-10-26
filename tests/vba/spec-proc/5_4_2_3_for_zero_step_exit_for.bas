' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 zero step with start not beyond end repeats until left explicitly
' MAX_STEPS: 100000
' EXPECT: iterations=4 i=2
Public Sub Main()
    Dim i As Long, c As Long
    For i = 2 To 5 Step 0
        c = c + 1
        If c = 4 Then Exit For
    Next i
    Debug.Print "iterations=" & CStr(c) & " i=" & CStr(i)
End Sub
