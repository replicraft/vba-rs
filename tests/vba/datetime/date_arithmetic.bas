' INV: type.date, op.add, op.sub
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  9 
' EXPECT:  1 
' EXPECT: True
Public Sub Main()
    Dim a As Date, b As Date
    a = DateSerial(2000, 1, 10)
    b = DateSerial(2000, 1, 1)
    Debug.Print a - b
    Debug.Print Day(b + 31)
    Debug.Print a > b
End Sub
