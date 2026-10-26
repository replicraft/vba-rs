' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 a For statement that is entered normally runs to its end, also when the loop is nested and entered again
' EXPECT: 12
Public Sub Main()
    Dim i As Long, k As Long, total As Long
    For k = 1 To 2
        For i = 1 To 3
            total = total + i
        Next i
    Next k
    Debug.Print CStr(total)
End Sub
