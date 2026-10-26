' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 method has implicit current-object parameter accessed through Me
' EXPECT: 5
' EXPECT: 6
'#MODULE Cls class
Public Value As Long
Public Sub Show()
    Debug.Print CStr(Me.Value)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Cls
    Dim b As New Cls
    a.Value = 5
    b.Value = 6
    a.Show
    b.Show
End Sub
