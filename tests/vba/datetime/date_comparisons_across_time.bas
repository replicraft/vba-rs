' INV: type.date, op.lt
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: TrueFalse
Public Sub Main()
    Dim a As Date, b As Date
    a = DateSerial(2000, 1, 1) + TimeSerial(10, 0, 0)
    b = DateSerial(2000, 1, 1) + TimeSerial(9, 59, 59)
    Debug.Print a > b; a = b
End Sub
