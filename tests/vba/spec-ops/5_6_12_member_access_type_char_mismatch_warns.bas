' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 a mismatching member type character is a warning by default (error under strict-spec, see docs/language/spec-coverage.md); the declared type wins
' EXPECT: 1
'#MODULE Box class
Public W As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.W = 1
    Debug.Print CStr(b.W$)
End Sub
