' INV: stmt.on-error
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1001 Badexploded
'#MODULE Bad class
Public Sub Boom()
    Err.Raise 1001, "Bad", "exploded"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Bad
    On Error Resume Next
    b.Boom
    Debug.Print Err.Number; Err.Source; Err.Description
End Sub
