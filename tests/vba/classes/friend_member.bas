' INV: decl.friend
' SOURCE: spec ms-vbal 5.3.1.1 Friend procedure scope
' EXPECT:  3 
'#MODULE Box class
Friend Function Secret() As Long
    Secret = 3
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    Debug.Print b.Secret()
End Sub
