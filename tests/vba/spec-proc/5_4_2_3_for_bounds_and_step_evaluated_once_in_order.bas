' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 start, end and step are evaluated once, in order, before iterating
' EXPECT: start
' EXPECT: end
' EXPECT: step
' EXPECT: count=3 limit-now=10
Dim limit As Long
Function Start() As Long
    Debug.Print "start"
    Start = 1
End Function
Function Finish() As Long
    Debug.Print "end"
    Finish = limit
End Function
Function Stp() As Long
    Debug.Print "step"
    Stp = 1
End Function
Public Sub Main()
    Dim i As Long, c As Long
    limit = 3
    For i = Start() To Finish() Step Stp()
        c = c + 1
        limit = 10
    Next i
    Debug.Print "count=" & CStr(c) & " limit-now=" & CStr(limit)
End Sub
