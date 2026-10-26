' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef Object parameter with differently typed class variable: local copy, set back on normal return
' XFAIL: wrong pending
' EXPECT: inside-still-nothing=True
' EXPECT: after=1
Dim c As Collection
Sub Renew(ByRef o As Object)
    Set o = New Collection
    o.Add 1
    Debug.Print "inside-still-nothing=" & CStr(c Is Nothing)
End Sub
Public Sub Main()
    Renew c
    Debug.Print "after=" & CStr(c.Count)
End Sub
