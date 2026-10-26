' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 procedural module variables have module extent
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
Private counter As Long
Private Sub Tick()
    counter = counter + 1
    Debug.Print CStr(counter)
End Sub
Public Sub Main()
    Tick
    Tick
    Tick
End Sub
