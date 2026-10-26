' INV: func.year, type.date
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  100  9999  1899 
Public Sub Main()
    Debug.Print Year(DateSerial(100, 1, 1)); Year(DateSerial(9999, 12, 31)); Year(0)
End Sub
