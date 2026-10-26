' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 Set assigns an object reference
' EXPECT: 5
' EXPECT: True
' EXPECT: True
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As C1, b As C1
    Set a = New C1
    a.V = 5
    Set b = a
    Debug.Print CStr(b.V)
    Debug.Print CStr(a Is b)
    Set b = Nothing
    Debug.Print CStr(b Is Nothing)
End Sub
