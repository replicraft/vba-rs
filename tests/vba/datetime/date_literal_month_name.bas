' INV: lit.date
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1969  2  12 
Public Sub Main()
    Dim d As Date
    d = #February 12, 1969#
    Debug.Print Year(d); Month(d); Day(d)
End Sub
