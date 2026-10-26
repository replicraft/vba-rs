' INV: func.macid, func.macscript
' SOURCE: unverified the page documents MacID as Macintosh-only; error 5 on a non-Mac implementation is a decision, not documented
' ERROR: 5
Public Sub Main()
    Debug.Print MacID("TEXT")
End Sub
