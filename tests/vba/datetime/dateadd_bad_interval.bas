' INV: func.dateadd
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 5
Public Sub Main()
    Debug.Print DateAdd("zz", 1, Date)
End Sub
