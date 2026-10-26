' INV: vbal.6.1.1.8
' SOURCE: spec ms-vbal 6.1.1.8 VbFileAttribute values
' EXPECT: vbNormal=0
' EXPECT: vbReadOnly=1
' EXPECT: vbHidden=2
' EXPECT: vbSystem=4
' EXPECT: vbVolume=8
' EXPECT: vbDirectory=16
' EXPECT: vbArchive=32
' EXPECT: vbAlias=64
Public Sub Main()
    Debug.Print "vbNormal=" & CStr(vbNormal)
    Debug.Print "vbReadOnly=" & CStr(vbReadOnly)
    Debug.Print "vbHidden=" & CStr(vbHidden)
    Debug.Print "vbSystem=" & CStr(vbSystem)
    Debug.Print "vbVolume=" & CStr(vbVolume)
    Debug.Print "vbDirectory=" & CStr(vbDirectory)
    Debug.Print "vbArchive=" & CStr(vbArchive)
    Debug.Print "vbAlias=" & CStr(vbAlias)
End Sub
