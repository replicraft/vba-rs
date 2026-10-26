'#MODULE Main standard
Public Sub Main()
    Dim c As Counter, i As Long, t As Long, o As Object
    Set c = New Counter
    Set o = c
    For i = 1 To 8000
        c.Add i
        t = (t + c.Total + c.Count) Mod 1000003
        o.Add 1
        t = (t + o.Total) Mod 1000003
    Next
    Debug.Print t
End Sub
'#MODULE Counter class
Public Total As Long
Private mN As Long
Public Sub Add(ByVal n As Long)
    Total = (Total + n) Mod 100003
    mN = mN + 1
End Sub
Public Property Get Count() As Long
    Count = mN
End Property
