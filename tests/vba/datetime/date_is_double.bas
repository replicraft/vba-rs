' INV: type.date
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0 
' EXPECT:  36526 
' EXPECT:  12 
Public Sub Main()
    Dim d As Date
    d = DateSerial(1899, 12, 30)
    Debug.Print CDbl(d)
    d = DateSerial(2000, 1, 1)
    Debug.Print CDbl(d)
    d = 0.5
    Debug.Print Hour(d)
End Sub
