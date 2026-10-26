' INV: stmt.on-error
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Err.Raise, 5.4.4 error handling policies
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
